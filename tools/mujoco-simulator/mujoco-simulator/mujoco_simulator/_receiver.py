from collections.abc import Generator
from dataclasses import dataclass
from typing import Any

from mujoco import MjData
from mujoco_rust_server import SimulationServer

from mujoco_simulator.topics import ReceiveTopic


@dataclass
class ReceivedTopic:
    last_received: float
    topic: ReceiveTopic


class Receiver:
    def __init__(self, *topics: ReceiveTopic) -> None:
        self.__received_topics = [
            ReceivedTopic(last_received=float("-inf"), topic=topic)
            for topic in topics
        ]

    def receive_updates(
        self, *, server: SimulationServer, data: MjData
    ) -> Generator[Any]:
        time = data.time
        for topic in self.__received_topics:
            time_since_last_reception = time - topic.last_received
            if (
                time_since_last_reception
                >= topic.topic.update_interval.total_seconds()
            ):
                topic.last_published = time
                yield topic.topic.receive(server)
