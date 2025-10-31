from enum import Enum, auto

from . import booster_types, zed_types

class SimulationServer:
    def __new__(cls, bind_address: str) -> SimulationServer: ...
    async def stop(self) -> None: ...
    async def next_task(self) -> PySimulationTask: ...
    def register_scene(self, scene: bytes) -> None: ...
    def update_scene_state(self, state: bytes) -> None: ...

class PySimulationTask:
    async def respond(
        self,
        time: float,
        response: booster_types.LowState | zed_types.RGBDSensors,
    ) -> None: ...
    async def receive(self) -> booster_types.LowCommand: ...

class TaskName(Enum):
    ApplyLowCommand = auto()
    RequestLowState = auto()
    RequestRGBDSensors = auto()
    StepSimulation = auto()
    Reset = auto()
    Invalid = auto()

__all__ = [
    "PySimulationTask",
    "SimulationServer",
    "TaskName",
    "booster_types",
    "zed_types",
]
