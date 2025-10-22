from abc import ABC, abstractmethod
from datetime import timedelta
from typing import ClassVar

from mujoco import MjData, MjModel
from mujoco_rust_server import SimulationServer


class BaseTopic(ABC):
    name: ClassVar[str]
    update_interval: timedelta

    def __init__(self, update_interval: timedelta) -> None:
        self.update_interval = update_interval


class ReceiveTopic[T](BaseTopic):
    def receive(self, server: SimulationServer) -> T:
        raise NotImplementedError


class SendTopic[T](BaseTopic):
    @abstractmethod
    def compute(self, *, model: MjModel, data: MjData) -> T:
        raise NotImplementedError

    @abstractmethod
    def publish(
        self, *, server: SimulationServer, model: MjModel, data: MjData
    ) -> None:
        raise NotImplementedError
