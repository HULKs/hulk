import asyncio
import logging
import time
from datetime import timedelta

import click
from mujoco import MjData, MjModel, mj_forward, mj_resetData, mj_step
from mujoco_rust_server import SimulationServer, TaskName
from mujoco_rust_server.zed_types import RGBDSensors
from rich.logging import RichHandler

from mujoco_simulator.exceptions import UnknownTaskException
from mujoco_simulator.joint_actuator_info import joint_actuator_info_list
from mujoco_simulator.low_state import generate_low_state
from mujoco_simulator.position_control import RobotPositionControl
from mujoco_simulator.rate_logger import SimulationRateLogger
from mujoco_simulator.render import CameraRenderer
from mujoco_simulator.scene import (
    generate_scene_description_binary,
    generate_scene_state_json,
)


def reset_simulation(model: MjModel, data: MjData) -> None:
    logging.info("Resetting simulation")
    mj_resetData(model, data)
    mj_forward(model, data)


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
    rate_logger = SimulationRateLogger(log_interval=timedelta(seconds=5))
    logging.info("Starting simulation loop")
    dt = model.opt.timestep
    logging.info(f"Timestep: {1000 * dt}ms")

    target_time_factor = 1
    renderer = CameraRenderer(
        model=model, camera_name="camera", height=480, width=640
    )
    actuator_info_list = joint_actuator_info_list(model)
    position_control = RobotPositionControl(model, actuator_info_list)

    last_tick = time.time()
    while True:
        task = await server.next_task()
        match task.kind():
            case TaskName.RequestRGBDSensors:
                rgbd_sensors = request_rgbd_sensors(renderer, data)
                await task.respond(data.time, rgbd_sensors)
            case TaskName.RequestLowState:
                low_state = generate_low_state(data, actuator_info_list)
                await task.respond(data.time, low_state)
            case TaskName.ApplyLowCommand:
                if low_command := await task.receive():
                    position_control.apply_control(data, low_command)
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
                scene_description = generate_scene_description_binary(model)
                await task.respond(data.time, scene_description)
            case TaskName.RequestSceneState:
                scene_state = generate_scene_state_json(model, data)
                await task.respond(data.time, scene_state)
            case _:
                raise UnknownTaskException(task.kind())


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
