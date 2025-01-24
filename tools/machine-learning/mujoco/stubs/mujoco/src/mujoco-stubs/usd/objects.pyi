import abc
import mujoco
import numpy as np
from _typeshed import Incomplete
from pxr import Usd as Usd
from typing import Any, Sequence

class USDObject(abc.ABC, metaclass=abc.ABCMeta):
    stage: Incomplete
    model: Incomplete
    geom: Incomplete
    obj_name: Incomplete
    rgba: Incomplete
    geom_textures: Incomplete
    xform_path: Incomplete
    usd_xform: Incomplete
    transform_op: Incomplete
    scale_op: Incomplete
    last_visible_frame: int
    def __init__(self, stage: Usd.Stage, model: mujoco.MjModel, geom: mujoco.MjvGeom, obj_name: str, rgba: np.ndarray = ..., geom_textures: Sequence[tuple[str, mujoco.mjtTexture] | None] = ()) -> None: ...
    def attach_image_material(self, usd_mesh) -> None: ...
    def attach_solid_material(self, usd_mesh) -> None: ...
    def update(self, pos: np.ndarray, mat: np.ndarray, visible: bool, frame: int, scale: np.ndarray | None = None): ...
    def update_visibility(self, visible: bool, frame: int): ...
    def update_scale(self, scale: np.ndarray, frame: int): ...

class USDMesh(USDObject):
    dataid: Incomplete
    usd_mesh: Incomplete
    usd_prim: Incomplete
    texcoords: Incomplete
    def __init__(self, stage: Usd.Stage, model: mujoco.MjModel, geom: mujoco.MjvGeom, obj_name: str, dataid: int, rgba: np.ndarray = ..., geom_textures: Sequence[tuple[str, mujoco.mjtTexture] | None] = ()) -> None: ...

class USDPrimitiveMesh(USDObject):
    mesh_config: Incomplete
    prim_mesh: Incomplete
    usd_mesh: Incomplete
    usd_prim: Incomplete
    texcoords: Incomplete
    def __init__(self, mesh_config: dict[Any, Any], stage: Usd.Stage, model: mujoco.MjModel, geom: mujoco.MjvGeom, obj_name: str, rgba: np.ndarray = ..., geom_textures: Sequence[tuple[str, mujoco.mjtTexture] | None] = ()) -> None: ...
    def generate_primitive_mesh(self): ...

class USDTendon(USDObject):
    mesh_config: Incomplete
    tendon_parts: Incomplete
    usd_refs: Incomplete
    texcoords: Incomplete
    def __init__(self, mesh_config: dict[Any, Any], stage: Usd.Stage, model: mujoco.MjModel, geom: mujoco.MjvGeom, obj_name: str, rgba: np.ndarray = ..., geom_textures: Sequence[tuple[str, mujoco.mjtTexture] | None] = ()) -> None: ...
    def generate_primitive_mesh(self): ...
    def update(self, pos: np.ndarray, mat: np.ndarray, visible: bool, frame: int, scale: np.ndarray | None = None): ...
    def update_scale(self, scale: np.ndarray, frame: int): ...
