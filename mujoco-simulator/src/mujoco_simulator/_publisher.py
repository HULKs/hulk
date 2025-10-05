from dataclasses import dataclass

from mujoco import MjData, MjModel
from mujoco_rust_server import SimulationServer

from .topics._base_topic import BaseTopic


@dataclass
class PublishedTopic:
    last_published: float
    topic: BaseTopic


class Publisher:
    def __init__(self, *topics: BaseTopic) -> None:
        self.__published_topics = [
            PublishedTopic(last_published=float("-inf"), topic=topic)
            for topic in topics
        ]

    def check_for_updates(
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
