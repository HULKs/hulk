from __future__ import annotations

from dataclasses import dataclass

import numpy as np
from common_types import Side
from numpy.typing import NDArray
from robot_dimensions import (
    ANKLE_TO_SOLE,
    HIP_TO_KNEE,
    KNEE_TO_ANKLE,
    ROBOT_TO_LEFT_PELVIS,
    ROBOT_TO_RIGHT_PELVIS,
)
from transforms import (
    isometry_from_rotation,
    isometry_from_translation,
    rotation_from_axisangle,
)

from .inverse_kinematics import LegJoints


def left_pelvis_to_robot(angles: LegJoints) -> NDArray:
    rotation = (
        rotation_from_axisangle(np.pi / 4, 0, 0)
        @ rotation_from_axisangle(0, 0, -angles.hip_yaw_pitch)
        @ rotation_from_axisangle(-np.pi / 4, 0, 0)
    )
    return isometry_from_translation(
        ROBOT_TO_LEFT_PELVIS,
    ) @ isometry_from_rotation(rotation)


def right_pelvis_to_robot(angles: LegJoints) -> NDArray:
    rotation = (
        rotation_from_axisangle(-np.pi / 4, 0, 0)
        @ rotation_from_axisangle(0, 0, angles.hip_yaw_pitch)
        @ rotation_from_axisangle(np.pi / 4, 0, 0)
    )
    return isometry_from_translation(
        ROBOT_TO_RIGHT_PELVIS,
    ) @ isometry_from_rotation(rotation)


def hip_to_pelvis(angles: LegJoints) -> NDArray:
    return isometry_from_rotation(
        rotation_from_axisangle(angles.hip_roll, 0, 0),
    )


def thigh_to_hip(angles: LegJoints) -> NDArray:
    return isometry_from_rotation(
        rotation_from_axisangle(0, angles.hip_pitch, 0),
    )


def tibia_to_thigh(angles: LegJoints) -> NDArray:
    return isometry_from_translation(HIP_TO_KNEE) @ isometry_from_rotation(
        rotation_from_axisangle(0, angles.knee_pitch, 0),
    )


def ankle_to_tibia(angles: LegJoints) -> NDArray:
    return isometry_from_translation(KNEE_TO_ANKLE) @ isometry_from_rotation(
        rotation_from_axisangle(0, angles.ankle_pitch, 0),
    )


def foot_to_ankle(angles: LegJoints) -> NDArray:
    return isometry_from_rotation(
        rotation_from_axisangle(angles.ankle_roll, 0, 0),
    )


def sole_to_robot(angles: LegJoints) -> NDArray:
    return (
        left_pelvis_to_robot(angles)
        @ hip_to_pelvis(angles)
        @ thigh_to_hip(angles)
        @ tibia_to_thigh(angles)
        @ ankle_to_tibia(angles)
        @ foot_to_ankle(angles)
        @ isometry_from_translation(ANKLE_TO_SOLE)
    )


@dataclass
class RobotLegKinematics:
    pelvis_to_robot: NDArray
    hip_to_robot: NDArray
    thigh_to_robot: NDArray
    tibia_to_robot: NDArray
    ankle_to_robot: NDArray
    foot_to_robot: NDArray
    sole_to_robot: NDArray

    @staticmethod
    def from_legjoints(angles: LegJoints, side: Side) -> RobotLegKinematics:
        pelvis_to_robot = (
            left_pelvis_to_robot(angles)
            if side == Side.LEFT
            else right_pelvis_to_robot(angles)
        )
        hip_to_robot = pelvis_to_robot @ hip_to_pelvis(angles)
        thigh_to_robot = hip_to_robot @ thigh_to_hip(angles)
        tibia_to_robot = thigh_to_robot @ tibia_to_thigh(angles)
        ankle_to_robot = tibia_to_robot @ ankle_to_tibia(angles)
        foot_to_robot = ankle_to_robot @ foot_to_ankle(angles)
        sole_to_robot = foot_to_robot @ isometry_from_translation(ANKLE_TO_SOLE)

        return RobotLegKinematics(
            pelvis_to_robot,
            hip_to_robot,
            thigh_to_robot,
            tibia_to_robot,
            ankle_to_robot,
            foot_to_robot,
            sole_to_robot,
        )
