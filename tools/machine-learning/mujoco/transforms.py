from __future__ import annotations

import numpy as np
from numpy.typing import NDArray
from scipy.spatial.transform import Rotation


def rotation_from_euler(
    roll: float, pitch: float, yaw: float
) -> NDArray[np.float64]:
    return Rotation.from_euler(
        "xyz", [roll, pitch, yaw], degrees=False
    ).as_matrix()


def isometry_from_euler(
    roll: float, pitch: float, yaw: float
) -> NDArray[np.float64]:
    rotation = np.eye(4)
    rotation[:3, :3] = rotation_from_euler(roll, pitch, yaw)
    return rotation


def isometry_from_translation(
    translation: NDArray[np.float64],
) -> NDArray[np.float64]:
    isometry = np.eye(4)
    isometry[:3, 3] = translation
    return isometry


def translation_from_isometry(
    transform: NDArray[np.float64],
) -> NDArray[np.float64]:
    return transform[:3, 3]


def rotation_from_isometry(
    transform: NDArray[np.float64],
) -> NDArray[np.float64]:
    return transform[:3, :3]


def inverse(transform: NDArray[np.float64]) -> NDArray[np.float64]:
    return np.linalg.inv(transform)
