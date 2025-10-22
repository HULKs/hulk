import mujoco
import numpy as np
from _typeshed import Incomplete

class USDExporter:
    model: Incomplete
    height: Incomplete
    width: Incomplete
    max_geom: Incomplete
    output_directory: Incomplete
    output_directory_root: Incomplete
    light_intensity: Incomplete
    camera_names: Incomplete
    specialized_materials_file: Incomplete
    verbose: Incomplete
    frame_count: int
    updates: int
    geom_names: Incomplete
    geom_refs: Incomplete
    usd_lights: Incomplete
    usd_cameras: Incomplete
    def __init__(self, model: mujoco.MjModel, height: int = 480, width: int = 480, max_geom: int = 10000, output_directory: str = 'mujoco_usdpkg', output_directory_root: str = './', light_intensity: int = 10000, camera_names: list[str] | None = None, specialized_materials_file: str | None = None, verbose: bool = True) -> None: ...
    @property
    def usd(self): ...
    @property
    def scene(self): ...
    def update_scene(self, data: mujoco.MjData, scene_option: mujoco.MjvOption | None = None): ...
    def add_light(self, pos: list[float], intensity: int, radius: float | None = 1.0, color: np.ndarray | None = ..., obj_name: str | None = 'light_1', light_type: str | None = 'sphere'): ...
    def add_camera(self, pos: list[float], rotation_xyz: list[float], obj_name: str | None = 'camera_1'): ...
    def save_scene(self, filetype: str = 'usd'): ...
