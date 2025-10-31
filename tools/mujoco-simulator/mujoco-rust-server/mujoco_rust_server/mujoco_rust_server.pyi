from enum import Enum, auto

from . import booster_types, zed_types

class SimulationServer:
    def __new__(cls, bind_address: str) -> SimulationServer: ...
    async def stop(self) -> None: ...
    async def next_task(self) -> PySimulationTask: ...

class PySimulationTask:
    def kind(self) -> TaskName: ...
    async def respond(
        self,
        time: float,
        response: booster_types.LowState | zed_types.RGBDSensors | bytes | str | None,
    ) -> None: ...
    async def receive(self) -> booster_types.LowCommand: ...

class TaskName(Enum):
    ApplyLowCommand = auto()
    RequestLowState = auto()
    RequestRGBDSensors = auto()
    RequestSceneState = auto()
    RequestSceneDescription = auto()
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
