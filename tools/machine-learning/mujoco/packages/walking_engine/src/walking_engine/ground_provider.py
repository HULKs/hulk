import numpy as np
from numpy.typing import NDArray
from transforms import (
    forget_rotation,
    isometry_from_rotation,
    isometry_from_translation,
    rotation_from_euler,
    translation_from_isometry,
)

from .walking_types import Side


def get_ground_to_robot(
    imu_roll: float,
    imu_pitch: float,
    left_sole_to_robot: NDArray,
    right_sole_to_robot: NDArray,
    support_side: Side,
) -> NDArray:
    imu_orientation = isometry_from_rotation(
        np.linalg.inv(rotation_from_euler(imu_roll, imu_pitch, 0.0))
    )

    left_sole_horizontal_to_robot = (
        forget_rotation(left_sole_to_robot) @ imu_orientation
    )
    right_sole_horizontal_to_robot = (
        forget_rotation(right_sole_to_robot) @ imu_orientation
    )

    left_sole_in_robot = translation_from_isometry(
        left_sole_horizontal_to_robot
    )
    right_sole_in_robot = translation_from_isometry(
        right_sole_horizontal_to_robot
    )

    left_sole_to_right_sole = right_sole_in_robot - left_sole_in_robot

    ground_to_left_sole = isometry_from_translation(
        np.array([left_sole_to_right_sole[0], left_sole_to_right_sole[1], 0.0])
        / 2.0
    )
    ground_to_right_sole = isometry_from_translation(
        -np.array([left_sole_to_right_sole[0], left_sole_to_right_sole[1], 0.0])
        / 2.0
    )

    if support_side == Side.LEFT:
        return left_sole_horizontal_to_robot @ ground_to_left_sole
    else:
        return right_sole_horizontal_to_robot @ ground_to_right_sole
