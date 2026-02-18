import numpy as np
from _typeshed import Incomplete
from pxr import Usd as Usd

class USDCamera:
    stage: Incomplete
    usd_xform: Incomplete
    usd_camera: Incomplete
    usd_prim: Incomplete
    transform_op: Incomplete
    def __init__(self, stage: Usd.Stage, obj_name: str) -> None: ...
    def update(self, cam_pos: np.ndarray, cam_mat: np.ndarray, frame: int): ...
