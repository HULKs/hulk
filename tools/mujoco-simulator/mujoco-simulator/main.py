import asyncio
from dataclasses import asdict
import json
import logging
import time
from datetime import timedelta

import click
from mujoco import MjData, MjModel, mj_forward, mj_resetData, mj_step
from mujoco_rust_server import PySimulationTask, SimulationServer, TaskName
from mujoco_rust_server.booster_types import LowState
from mujoco_rust_server.zed_types import RGBDSensors
from rich.logging import RichHandler

from mujoco_simulator import (
    SceneExporter,
    get_control_input,
)
from mujoco_simulator._camera_render import CameraRenderer
from mujoco_simulator._scene_exporter import export_scene, serialize
from mujoco_simulator.topics._low_state_topic import generate_low_state
from mujoco_simulator.topics._scene_topic import (
    SceneStateTopic,
    get_scene_state,
)


class SimulationRateLogger:
    def __init__(self, log_rate: timedelta) -> None:
        self.log_rate = log_rate
        self.last_log = None
        self.steps_since_last_log = 0

    def step(self) -> None:
        self.steps_since_last_log += 1
        now = time.time()
        if self.last_log is None:
            self.last_log = now

        if now - self.last_log >= self.log_rate.total_seconds():
            rate = self.steps_since_last_log / self.log_rate.total_seconds()
            logging.info(f"Simulation [steps/second]: {int(rate)}")
            self.steps_since_last_log = 0
            self.last_log = now


def reset_simulation(model: MjModel, data: MjData) -> None:
    logging.info("Resetting simulation")
    mj_resetData(model, data)
    mj_forward(model, data)


def request_low_state(model: MjModel, data: MjData) -> LowState:
    return generate_low_state(model, data)


def request_rgbd_sensors(renderer: CameraRenderer, data: MjData) -> RGBDSensors:
    image = renderer.render(data)
    return RGBDSensors(
        data.time,
        image.rgb.flatten().tobytes(),
        image.depth.flatten().tobytes(),
        image.height(),
        image.width(),
    )


async def run_simulation(
    server: SimulationServer, model: MjModel, data: MjData
) -> None:
    rate_logger = SimulationRateLogger(log_rate=timedelta(seconds=5))
    logging.info("Starting simulation loop")
    dt = model.opt.timestep
    logging.info(f"Timestep: {1000 * dt}ms")

    target_time_factor = 1
    renderer = CameraRenderer(
        model=model, camera_name="camera", height=480, width=640
    )

    last_tick = time.time()
    while True:
        task = await server.next_task()
        match task.kind():
            case TaskName.RequestRGBDSensors:
                rgbd_sensors = request_rgbd_sensors(renderer, data)
                await task.respond(data.time, rgbd_sensors)
            case TaskName.RequestLowState:
                low_state = request_low_state(model, data)
                await task.respond(data.time, low_state)
            case TaskName.ApplyLowCommand:
                if low_command := await task.receive():
                    data.ctrl[:] = get_control_input(model, data, low_command)
            case TaskName.Reset:
                reset_simulation(model, data)
            case TaskName.StepSimulation:
                now = time.time()
                await asyncio.sleep(
                    max(0, dt * target_time_factor - (now - last_tick))
                )
                mj_step(model, data)
                rate_logger.step()
                last_tick = time.time()
                await task.respond(data.time, None)
            case TaskName.RequestSceneDescription:
                scene_description = export_scene(model)
                await task.respond(data.time, serialize(scene_description))
            case TaskName.RequestSceneState:
                scene_state = get_scene_state(model, data)
                await task.respond(data.time, json.dumps(asdict(scene_state)))
            case _:
                raise ValueError(f"Unknown task: {task.kind()}")


async def main(*, bind_address: str) -> None:
    logging.basicConfig(
        level="DEBUG",
        format="%(message)s",
        datefmt="[%X]",
        handlers=[RichHandler(rich_tracebacks=True)],
    )
    model = MjModel.from_xml_path("K1/K1.xml")
    data = MjData(model)
    mj_resetData(model, data)
    mj_forward(model, data)

    server = SimulationServer(bind_address)
    try:
        await run_simulation(server, model, data)
    except Exception as e:
        logging.exception(e)
    finally:
        await server.stop()


@click.command()
@click.option(
    "--bind-address", default="0.0.0.0:8000", help="Bind address for the server"
)
def cli(*, bind_address: str) -> None:
    asyncio.run(main(bind_address=bind_address))


if __name__ == "__main__":
    cli()
