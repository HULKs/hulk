import numpy as np


def __immutable(array: np.ndarray) -> np.ndarray:
    array.setflags(write=False)
    return array


ROBOT_TO_LEFT_PELVIS = __immutable(np.array([0.0, 0.05, 0.0]))
ROBOT_TO_RIGHT_PELVIS = __immutable(np.array([0.0, -0.05, 0.0]))
HIP_TO_KNEE = __immutable(np.array([0.0, 0.0, -0.1]))
KNEE_TO_ANKLE = __immutable(np.array([0.0, 0.0, -0.1029]))
ANKLE_TO_SOLE = __immutable(np.array([0.0, 0.0, -0.04519]))
