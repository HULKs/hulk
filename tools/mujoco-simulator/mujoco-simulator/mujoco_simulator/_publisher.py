from dataclasses import dataclass

from mujoco import MjData, MjModel
from mujoco_rust_server import SimulationServer

from mujoco_simulator.topics import SendTopic


@dataclass
class PublishedTopic:
    last_published: float
    topic: SendTopic


class Publisher:
    def __init__(self, *topics: SendTopic) -> None:
        self.__published_topics = [
            PublishedTopic(last_published=float("-inf"), topic=topic)
            for topic in topics
        ]

    def send_updates(
        self, *, server: SimulationServer, model: MjModel, data: MjData
    ) -> None:
        time = data.time
        for topic in self.__published_topics:
            time_since_last_update = time - topic.last_published
            if (
                time_since_last_update
                >= topic.topic.update_interval.total_seconds()
            ):
                topic.topic.publish(server=server, model=model, data=data)
                topic.last_published = time

    def should_expect_low_command_update(self, data: MjData) -> bool:
        for published_topic in self.__published_topics:
            if (
                published_topic.topic.name == "low_state"
                and published_topic.last_published == data.time
            ):
                return True

        return False
