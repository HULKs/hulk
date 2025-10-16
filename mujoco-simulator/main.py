import logging
import time
from datetime import timedelta

import click
from mujoco import MjData, MjModel, mj_resetData, mj_step
from mujoco_rust_server import ServerCommand, SimulationServer
from rich.logging import RichHandler

from mujoco_simulator import (
    SceneExporter,
    get_control_input,
)
from mujoco_simulator._publisher import Publisher
from mujoco_simulator.topics import CameraTopic, LowStateTopic


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
    scene_exporter = SceneExporter(
        server=server,
        model=model,
    )
    publisher = Publisher(
        LowStateTopic(update_interval=timedelta(milliseconds=10)),
        CameraTopic(
            update_interval=timedelta(milliseconds=10),
            model=model,
        ),
    )

    if gui:
        raise NotImplementedError

    while True:
        start = time.time()
        command = server.receive_simulation_command()
        handle_server_command(command, model, data)

        mj_step(model, data)
        publisher.check_for_updates(server=server, model=model, data=data)
        scene_exporter.publish(data)

        # TODO(oleflb): issue with SIGINT when connected via websocket
        if publisher.should_expect_low_command_update(data):
            received_command = server.receive_low_command_blocking()
            data.ctrl[:] = get_control_input(model, data, received_command)

        update_duration = time.time() - start
        time.sleep(max(0, dt / target_time_factor - update_duration))


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
        handlers=[RichHandler(rich_tracebacks=True)],
    )
    server = SimulationServer(bind_address)

    try:
        run_simulation(server, gui=gui)
    finally:
        server.stop()


if __name__ == "__main__":
    main()
