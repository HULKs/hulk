import numpy as np
from _typeshed import Incomplete
from pxr import Usd as Usd

class USDSphereLight:
    stage: Incomplete
    usd_xform: Incomplete
    usd_light: Incomplete
    usd_prim: Incomplete
    translate_op: Incomplete
    def __init__(self, stage: Usd.Stage, obj_name: str, radius: float | None = 0.3) -> None: ...
    def update(self, pos: np.ndarray, intensity: int, color: np.ndarray, frame: int): ...

class USDDomeLight:
    stage: Incomplete
    usd_xform: Incomplete
    usd_light: Incomplete
    usd_prim: Incomplete
    def __init__(self, stage: Usd.Stage, obj_name: str) -> None: ...
    def update(self, intensity: int, color: np.ndarray): ...
