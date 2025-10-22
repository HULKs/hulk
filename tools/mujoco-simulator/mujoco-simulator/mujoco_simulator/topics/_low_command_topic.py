from typing import override

from mujoco_rust_server import SimulationServer
from mujoco_rust_server.booster_types import LowCommand

from ._base_topic import ReceiveTopic


class LowCommandTopic(ReceiveTopic):
    name = "low_command"

    @override
    def receive(self, server: SimulationServer) -> LowCommand:
        return server.receive_low_command_blocking()
