import logging
import time

import click
from mujoco import MjData, MjModel, mj_resetData, mj_step
from mujoco_rust_server import ServerCommand, SimulationServer

from mujoco_simulator import (
    CameraEncoder,
    H264Encoder,
    SceneExporter,
    get_control_input,
)
from mujoco_simulator._utils import generate_low_state


def handle_server_command(
    command: ServerCommand | None, model: MjModel, data: MjData
) -> None:
    if command is None:
        return
    match command:
        case ServerCommand.Reset:
            logging.info("Resetting simulation")
            mj_resetData(model, data)


def run_simulation(server: SimulationServer, *, gui: bool) -> None:
    model = MjModel.from_xml_path("K1/K1.xml")
    data = MjData(model)
    dt = model.opt.timestep
    logging.info(f"Timestep: {1000 * dt}ms")

    target_time_factor = 1
    current_low_command = None
    scene_exporter = SceneExporter(
        server=server,
        model=model,
    )
    # encoder = CameraEncoder(model=model, camera_name="camera")
    # h264_encoder = H264Encoder(width=640, height=480)
    if gui:
        raise NotImplementedError

    while True:
        start = time.time()
        command = server.receive_simulation_command()
        handle_server_command(command, model, data)

        data.ctrl[:] = get_control_input(model, data, current_low_command)
        mj_step(model, data)

        server.send_low_state(data.time, generate_low_state(model, data))
        received_command = server.receive_low_command()
        if received_command is not None:
            current_low_command = received_command

        # camera_frame = encoder.render(data)
        # encoded_updated = h264_encoder.encode_frame(camera_frame)
        # server.send_camera_frame(encoded_updated)

        scene_exporter.publish(data)
        update_duration = time.time() - start
        time.sleep(max(0, dt / target_time_factor - update_duration))

    # with viewer.launch_passive(model, data) as handle:
    #     while handle.is_running:
    #         start = time.time()
    #         command = server.receive_simulation_command()
    #         handle_server_command(command, model, data)

    #         with handle.lock():
    #             data.ctrl[:] = get_control_input(
    #                 model, data, current_low_command
    #             )
    #             mj_step(model, data)
    #         handle.sync()

    #         server.send_low_state(generate_low_state(model, data))
    #         received_command = server.receive_low_command()
    #         if received_command is not None:
    #             current_low_command = received_command
    #         encoded_camera_frame = encoder.render(data)
    #         server.send_camera_frame(encoded_camera_frame)

    #         scene_exporter.publish(data)
    #         update_duration = time.time() - start
    #         time.sleep(max(0, dt / target_time_factor - update_duration))


@click.command()
@click.option(
    "--bind-address", default="0.0.0.0:8000", help="Bind address for the server"
)
@click.option("--gui", is_flag=True, default=False, help="Enable GUI")
def main(*, bind_address: str, gui: bool) -> None:
    logging.basicConfig(
    level="DEBUG",
    format="%(message)s",
    datefmt="[%X]",
    handlers=[RichHandler(rich_tracebacks=True)]
)
    server = SimulationServer(bind_address)

    try:
        run_simulation(server, gui=gui)
    finally:
        server.stop()


if __name__ == "__main__":
    main()
