import logging
import time
from datetime import timedelta
import asyncio

import click
from mujoco import MjData, MjModel, mj_forward, mj_resetData, mj_step
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


async def run_simulation(
    server: SimulationServer, model: MjModel, data: MjData
) -> None:
    dt = model.opt.timestep
    logging.info(f"Timestep: {1000 * dt}ms")

    target_time_factor = 1
    scene_exporter = SceneExporter(
        server=server,
        model=model,
    )
    # publisher = Publisher(
    #     LowStateTopic(update_interval=timedelta(milliseconds=10)),
    #     CameraTopic(
    #         update_interval=timedelta(milliseconds=10),
    #         model=model,
    #     ),
    #     SceneStateTopic(update_interval=timedelta(milliseconds=2)),
    # )
    # receiver = Receiver(
    #     LowCommandTopic(update_interval=timedelta(milliseconds=10)),
    # )

    while True:
        task = await server.next_task(data.time)
        match task:
            case something:
                logging.info(f"Received task: {something}")
        quit()


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
    import threading

    logging.info(f"Main thread id: {threading.get_ident()}")
    for thread in threading.enumerate():
        logging.info(f"Thread: {thread.name}, id: {thread.ident}")

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
