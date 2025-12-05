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

class Material:
    @staticmethod
    def rgba(rgba: list[float]) -> Material: ...
    @staticmethod
    def pbr(material_index: int) -> Material: ...

class PbrMaterial:
    def __init__(
        self,
        rgba: list[float],
        reflectance: float,
        shininess: float,
        specular: float,
    ) -> None: ...

class Geom:
    id: int
    material: Material
    pos: list[float]
    quat: list[float]

    @staticmethod
    def mesh(
        index: int,
        mesh_index: int,
        material: Material,
        pos: list[float],
        quat: list[float],
    ) -> Geom: ...
    @staticmethod
    def sphere(
        index: int,
        radius: str,
        material: Material,
        pos: list[float],
        quat: list[float],
    ) -> Geom: ...
    @staticmethod
    def box(
        index: int,
        extent: list[float],
        material: Material,
        pos: list[float],
        quat: list[float],
    ) -> Geom: ...
    @staticmethod
    def plane(
        index: int,
        normal: list[float],
        material: Material,
        pos: list[float],
        quat: list[float],
    ) -> Geom: ...
    @staticmethod
    def cylinder(
        index: int,
        radius: float,
        half_height: float,
        material: Material,
        pos: list[float],
        quat: list[float],
    ) -> Geom: ...

class Light:
    name: str | None
    pos: list[float]
    dir: list[float]

class SceneDescription:
    meshes: dict[int, SceneMesh]
    materials: dict[int, PbrMaterial]
    lights: list[Light]
    bodies: dict[int, Body]
    geoms: dict[int, Geom]

class SceneMesh:
    vertices: list[list[float]]
    faces: list[list[int]]

class SceneUpdate:
    time: float
    bodies: dict[int, BodyUpdate]

class Texture:
    name: str | None
    width: int
    height: int
    rgb: list[int]

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
    "Texture",
    "booster_types",
    "zed_types",
]
