import asyncio
import logging
import time
from datetime import timedelta

import click
import numpy as np
from mujoco import MjData, MjModel, mj_forward, mj_resetData, mj_step
from mujoco_rust_server import SimulationServer, TaskName
from mujoco_rust_server.ros2_types import CameraInfo, Image
from rich.logging import RichHandler

from mujoco_simulator.exceptions import UnknownTaskException
from mujoco_simulator.joint_actuator_info import joint_actuator_info_list
from mujoco_simulator.low_state import generate_low_state
from mujoco_simulator.position_control import RobotPositionControl
from mujoco_simulator.rate_logger import SimulationRateLogger
from mujoco_simulator.render import CameraRenderer
from mujoco_simulator.scene import (
    generate_scene_description,
    generate_scene_state,
)


def reset_simulation(model: MjModel, data: MjData) -> None:
    logging.info("Resetting simulation")
    mj_resetData(model, data)
    mj_forward(model, data)


def request_image(renderer: CameraRenderer, data: MjData) -> Image:
    image = renderer.render(data)
    return Image(
        data.time,
        image.rgb.flatten().tobytes(),
        image.height(),
        image.width(),
    )


def request_camera_info(
    renderer: CameraRenderer, data: MjData, model: MjModel
) -> CameraInfo:
    fov = model.vis.global_.fovy

    focal_scaling = (
        (1.0 / np.tan(np.deg2rad(fov) / 2)) * renderer.viewport.height / 2.0
    )
    optical_center_x = (renderer.viewport.width - 1) / 2.0
    optical_center_y = (renderer.viewport.height - 1) / 2.0

    return CameraInfo(
        data.time,
        renderer.viewport.height,
        renderer.viewport.width,
        focal_scaling,
        focal_scaling,
        optical_center_x,
        optical_center_y,
    )


async def run_simulation(
    server: SimulationServer, model: MjModel, data: MjData
) -> None:
    rate_logger = SimulationRateLogger(log_interval=timedelta(seconds=5))
    scene_description = generate_scene_description(model)

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
            case TaskName.RequestImage:
                image = request_image(renderer, data)
                await task.respond(data.time, image)
            case TaskName.RequestCameraInfo:
                camera_info = request_camera_info(renderer, data, model)
                await task.respond(data.time, camera_info)
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
                await task.respond(data.time, scene_description)
            case TaskName.RequestSceneState:
                scene_state = generate_scene_state(model, data)
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
