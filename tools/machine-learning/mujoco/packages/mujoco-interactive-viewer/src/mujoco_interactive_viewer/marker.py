from dataclasses import dataclass

import mujoco
import numpy as np
from numpy.typing import NDArray


@dataclass
class Marker:
    kind: mujoco.mjtGeom
    size: NDArray[np.float64]
    position: NDArray[np.float64]
    rotation_matrix: NDArray[np.float64]
    rgba: NDArray[np.float32]
