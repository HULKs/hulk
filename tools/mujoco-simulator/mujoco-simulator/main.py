import asyncio
import logging
import time

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
from mujoco_simulator.topics._low_state_topic import generate_low_state


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
        image.rgb.flatten(),
        image.depth.flatten(),
        image.height(),
        image.width(),
    )


async def run_simulation(
    server: SimulationServer, model: MjModel, data: MjData
) -> None:
    logging.info("Starting simulation loop")
    dt = model.opt.timestep
    logging.info(f"Timestep: {1000 * dt}ms")

    target_time_factor = 1
    _ = SceneExporter(
        server=server,
        model=model,
    )
    renderer = CameraRenderer(
        model=model, camera_name="camera", height=480, width=640
    )

    last_tick = time.time()
    while True:
        logging.info("Waiting for next task")
        task = await server.next_task()
        logging.info(f"Received task: {task.kind()}")
        match task.kind():
            case TaskName.RequestRGBDSensors:
                rgbd_sensors = request_rgbd_sensors(renderer, data)
                await task.respond(data.time, rgbd_sensors)
            case TaskName.RequestLowState:
                low_state = request_low_state(model, data)
                await task.respond(data.time, low_state)
            case TaskName.ApplyLowCommand:
                low_command = await task.receive()
                data.ctrl[:] = get_control_input(model, data, low_command)
            case TaskName.Reset:
                reset_simulation(model, data)
            case TaskName.StepSimulation:
                now = time.time()
                await asyncio.sleep(
                    max(0, dt * target_time_factor - (now - last_tick))
                )
                mj_step(model, data)
                last_tick = time.time()
                await task.respond(data.time, None)
            case _:
                logging.warning(f"Unknown task: {task.kind()}")


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
