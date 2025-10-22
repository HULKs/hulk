import logging
import time
from datetime import timedelta

import click
from mujoco import MjData, MjModel, mj_resetData, mj_step, mj_forward
from mujoco_rust_server import ServerCommand, SimulationServer
from rich.logging import RichHandler

from mujoco_simulator import (
    Publisher,
    Receiver,
    SceneExporter,
    get_control_input,
)
from mujoco_simulator.topics import (
    CameraTopic,
    LowCommandTopic,
    LowStateTopic,
    SceneStateTopic,
)


def handle_server_command(
    command: ServerCommand | None, model: MjModel, data: MjData
) -> None:
    if command is None:
        return
    match command:
        case ServerCommand.Reset:
            logging.info("Resetting simulation")
            mj_resetData(model, data)
            mj_forward(model, data)


def run_simulation(
    server: SimulationServer, model: MjModel, data: MjData
) -> None:
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
        SceneStateTopic(update_interval=timedelta(milliseconds=2)),
    )
    receiver = Receiver(
        LowCommandTopic(update_interval=timedelta(milliseconds=10)),
    )

    while True:
        start = time.time()
        command = server.receive_simulation_command()
        handle_server_command(command, model, data)

        mj_step(model, data)
        publisher.send_updates(server=server, model=model, data=data)
        # TODO(oleflb): possible deadlock if client connects
        #               but does not receive sensor data
        for reception in receiver.receive_updates(server=server, data=data):
            if isinstance(reception, LowCommand):
                data.ctrl[:] = get_control_input(model, data, received_command)

        update_duration = time.time() - start
        time.sleep(max(0, dt / target_time_factor - update_duration))


@click.command()
@click.option(
    "--bind-address", default="0.0.0.0:8000", help="Bind address for the server"
)
def main(*, bind_address: str) -> None:
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

    server = SimulationServer(
        bind_address,
        LowStateTopic(timedelta(0)).compute(model=model, data=data),
    )

    try:
        run_simulation(server, model, data)
    finally:
        server.stop()


if __name__ == "__main__":
    main()
