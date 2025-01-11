from __future__ import annotations

from dataclasses import dataclass

import numpy as np
from numpy.typing import NDArray
from scipy.spatial.transform import Rotation


@dataclass
class Pose2:
    x: float = 0.0
    y: float = 0.0
    theta: float = 0.0

    def __add__(self, other: Pose2) -> Pose2:
        return Pose2(
            x=self.x + other.x,
            y=self.y + other.y,
            theta=self.theta + other.theta,
        )

    def __mul__(self, other: float) -> Pose2:
        return Pose2(
            x=self.x * other,
            y=self.y * other,
            theta=self.theta * other,
        )

    def __sub__(self, other: Pose2) -> Pose2:
        return Pose2(
            x=self.x - other.x,
            y=self.y - other.y,
            theta=self.theta - other.theta,
        )


def rotation_from_axisangle(
    x: np.floating,
    y: np.floating,
    z: np.floating,
) -> NDArray[np.float64]:
    return Rotation.from_rotvec([x, y, z], degrees=False).as_matrix()


def rotation_from_euler(
    roll: float,
    pitch: float,
    yaw: float,
) -> NDArray[np.float64]:
    return Rotation.from_euler(
        "xyz",
        [roll, pitch, yaw],
        degrees=False,
    ).as_matrix()


def isometry_from_euler(
    roll: float,
    pitch: float,
    yaw: float,
) -> NDArray[np.float64]:
    rotation = np.eye(4)
    rotation[:3, :3] = rotation_from_euler(roll, pitch, yaw)
    return rotation


def isometry_from_rotation(
    rotation: NDArray[np.float64],
) -> NDArray[np.float64]:
    isometry = np.eye(4)
    isometry[:3, :3] = rotation
    return isometry


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


def forget_rotation(transform: NDArray[np.float64]) -> NDArray[np.float64]:
    result = np.copy(transform)
    result[:3, :3] = np.eye(3)
    return result


def inverse(transform: NDArray[np.float64]) -> NDArray[np.float64]:
    return np.linalg.inv(transform)


def quaternion_from_isometry(
    transform: NDArray[np.float64],
) -> NDArray[np.float64]:
    return Rotation.from_matrix(transform[:3, :3]).as_quat(scalar_first=True)


def project_isometry_in_z_to_pose2(transform: NDArray[np.float64]) -> Pose2:
    xy_translation = transform[:2, -1]
    xy_rotation = transform[:2, 0]
    theta = np.atan2(xy_rotation[1], xy_rotation[0])
    return Pose2(x=xy_translation[0], y=xy_translation[1], theta=theta)
