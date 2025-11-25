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
        response: booster_types.LowState
        | zed_types.RGBDSensors
        | bytes
        | str
        | None,
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

class Body:
    id: int
    parent: str | None
    geoms: list[Geom]

class BodyUpdate:
    pos: list[float]
    quat: list[float]

class Geom:
    name: str
    mesh: str | None
    rgba: list[float]
    pos: list[float]
    quat: list[float]

class Light:
    name: str | None
    pos: list[float]
    dir: list[float]

class SceneDescription:
    meshes: dict[str, SceneMesh]
    lights: list[Light]
    bodies: dict[str, Body]

class SceneMesh:
    vertices: list[list[float]]
    faces: list[list[int]]

class SceneUpdate:
    time: float
    bodies: dict[str, BodyUpdate]

__all__ = [
    "Body",
    "BodyUpdate",
    "Geom",
    "Light",
    "PySimulationTask",
    "SceneDescription",
    "SceneMesh",
    "SceneUpdate",
    "SimulationServer",
    "TaskName",
    "booster_types",
    "zed_types",
]
