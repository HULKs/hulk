from dataclasses import dataclass

import numpy as np
from numpy.typing import NDArray
from transforms import (
    inverse,
    isometry_from_euler,
    isometry_from_translation,
    rotation_from_euler,
    rotation_from_isometry,
    translation_from_isometry,
)

ROBOT_TO_LEFT_PELVIS = np.array([0.0, 0.05, 0.0])
ROBOT_TO_RIGHT_PELVIS = np.array([0.0, -0.05, 0.0])
LEFT_HIP_TO_LEFT_KNEE = np.array([0.0, 0.0, -0.1])
LEFT_KNEE_TO_LEFT_ANKLE = np.array([0.0, 0.0, -0.1029])


@dataclass
class LegJoints:
    hip_yaw_pitch: float
    hip_roll: float
    hip_pitch: float
    knee_pitch: float
    ankle_pitch: float
    ankle_roll: float

    def to_numpy(self) -> NDArray[np.float64]:
        return np.array(
            [
                self.hip_yaw_pitch,
                self.hip_roll,
                self.hip_pitch,
                self.knee_pitch,
                self.ankle_pitch,
                self.ankle_roll,
            ]
        )


def leg_angles(
    left_foot: NDArray[np.float64],
    right_foot: NDArray[np.float64],
) -> tuple[LegJoints, LegJoints]:
    ratio = 0.5
    robot_to_left_pelvis = isometry_from_euler(
        -np.pi / 4, 0.0, 0.0
    ) @ isometry_from_translation(-ROBOT_TO_LEFT_PELVIS)
    robot_to_right_pelvis = isometry_from_euler(
        np.pi / 4, 0.0, 0.0
    ) @ isometry_from_translation(-ROBOT_TO_RIGHT_PELVIS)

    left_foot_to_left_pelvis = robot_to_left_pelvis @ left_foot
    right_foot_to_right_pelvis = robot_to_right_pelvis @ right_foot
    vector_left_foot_to_left_pelvis = translation_from_isometry(
        inverse(left_foot_to_left_pelvis)
    )
    vector_right_foot_to_right_pelvis = translation_from_isometry(
        inverse(right_foot_to_right_pelvis)
    )

    left_foot_roll_in_pelvis = np.arctan2(
        vector_left_foot_to_left_pelvis[1], vector_left_foot_to_left_pelvis[2]
    )
    right_foot_roll_in_pelvis = np.arctan2(
        vector_right_foot_to_right_pelvis[1],
        vector_right_foot_to_right_pelvis[2],
    )

    left_foot_pitch_2_in_pelvis = np.arctan2(
        vector_left_foot_to_left_pelvis[0],
        np.linalg.norm(vector_left_foot_to_left_pelvis[1:3]),
    )
    right_foot_pitch_2_in_pelvis = np.arctan2(
        vector_right_foot_to_right_pelvis[0],
        np.linalg.norm(vector_right_foot_to_right_pelvis[1:3]),
    )

    left_hip_rotation_c1 = (
        rotation_from_isometry(left_foot_to_left_pelvis)
        @ rotation_from_euler(-left_foot_roll_in_pelvis, 0.0, 0.0)
        @ rotation_from_euler(0.0, left_foot_pitch_2_in_pelvis, 0.0)
        @ np.array([0.0, 1.0, 0.0])
    )
    right_hip_rotation_c1 = (
        rotation_from_isometry(right_foot_to_right_pelvis)
        @ rotation_from_euler(-right_foot_roll_in_pelvis, 0.0, 0.0)
        @ rotation_from_euler(0.0, right_foot_pitch_2_in_pelvis, 0.0)
        @ np.array([0.0, 1.0, 0.0])
    )

    left_hip_yaw_pitch = -np.arctan2(
        -left_hip_rotation_c1[0], left_hip_rotation_c1[1]
    )
    right_hip_yaw_pitch = np.arctan2(
        -right_hip_rotation_c1[0], right_hip_rotation_c1[1]
    )
    left_hip_yaw_pitch_combined = (
        left_hip_yaw_pitch * ratio + right_hip_yaw_pitch * (1 - ratio)
    )

    left_pelvis_to_left_hip = isometry_from_euler(
        0.0, 0.0, left_hip_yaw_pitch_combined
    )
    left_foot_to_left_hip = left_pelvis_to_left_hip @ left_foot_to_left_pelvis
    right_pelvis_to_right_hip = isometry_from_euler(
        0.0, 0.0, -left_hip_yaw_pitch_combined
    )
    right_foot_to_right_hip = (
        right_pelvis_to_right_hip @ right_foot_to_right_pelvis
    )

    vector_left_hip_to_left_foot = translation_from_isometry(
        left_foot_to_left_hip
    )
    vector_right_hip_to_right_foot = translation_from_isometry(
        right_foot_to_right_hip
    )

    left_hip_roll_in_hip = -np.arctan2(
        -vector_left_hip_to_left_foot[1], -vector_left_hip_to_left_foot[2]
    )
    right_hip_roll_in_hip = -np.arctan2(
        -vector_right_hip_to_right_foot[1], -vector_right_hip_to_right_foot[2]
    )

    left_hip_pitch_minus_alpha = np.arctan2(
        -vector_left_hip_to_left_foot[0],
        -np.linalg.norm(vector_left_hip_to_left_foot[1:3])
        * np.sign(vector_left_hip_to_left_foot[2]),
    )
    right_hip_pitch_minus_alpha = np.arctan2(
        -vector_right_hip_to_right_foot[0],
        -np.linalg.norm(vector_right_hip_to_right_foot[1:3])
        * np.sign(vector_right_hip_to_right_foot[2]),
    )

    left_foot_rotation_c2 = (
        rotation_from_euler(0.0, -left_hip_pitch_minus_alpha, 0.0)
        @ rotation_from_euler(-left_hip_roll_in_hip, 0.0, 0.0)
        @ rotation_from_isometry(left_foot_to_left_hip)
        @ np.array([0.0, 0.0, 1.0])
    )
    right_foot_rotation_c2 = (
        rotation_from_euler(0.0, -right_hip_pitch_minus_alpha, 0.0)
        @ rotation_from_euler(-right_hip_roll_in_hip, 0.0, 0.0)
        @ rotation_from_isometry(right_foot_to_right_hip)
        @ np.array([0.0, 0.0, 1.0])
    )

    upper_leg = np.abs(LEFT_HIP_TO_LEFT_KNEE[2])
    lower_leg = np.abs(LEFT_KNEE_TO_LEFT_ANKLE[2])
    left_height = np.linalg.norm(
        translation_from_isometry(left_foot_to_left_hip)
    )
    right_height = np.linalg.norm(
        translation_from_isometry(right_foot_to_right_hip)
    )

    left_cos_minus_apha = (upper_leg**2 + left_height**2 - lower_leg**2) / (
        2 * upper_leg * left_height
    )
    right_cos_minus_apha = (upper_leg**2 + right_height**2 - lower_leg**2) / (
        2 * upper_leg * right_height
    )
    left_cos_minus_beta = (lower_leg**2 + left_height**2 - upper_leg**2) / (
        2 * lower_leg * left_height
    )
    right_cos_minus_beta = (lower_leg**2 + right_height**2 - upper_leg**2) / (
        2 * lower_leg * right_height
    )
    left_alpha = -np.arccos(np.clip(left_cos_minus_apha, -1.0, 1.0))
    right_alpha = -np.arccos(np.clip(right_cos_minus_apha, -1.0, 1.0))
    left_beta = -np.arccos(np.clip(left_cos_minus_beta, -1.0, 1.0))
    right_beta = -np.arccos(np.clip(right_cos_minus_beta, -1.0, 1.0))

    left_leg = LegJoints(
        hip_yaw_pitch=left_hip_yaw_pitch_combined,
        hip_roll=left_hip_roll_in_hip + np.pi / 4.0,
        hip_pitch=left_hip_pitch_minus_alpha + left_alpha,
        knee_pitch=-left_alpha - left_beta,
        ankle_pitch=np.arctan2(
            left_foot_rotation_c2[0], left_foot_rotation_c2[2]
        )
        + left_beta,
        ankle_roll=np.arcsin(-left_foot_rotation_c2[1]),
    )
    right_leg = LegJoints(
        hip_yaw_pitch=left_hip_yaw_pitch_combined,
        hip_roll=right_hip_roll_in_hip - np.pi / 4.0,
        hip_pitch=right_hip_pitch_minus_alpha + right_alpha,
        knee_pitch=-right_alpha - right_beta,
        ankle_pitch=np.arctan2(
            right_foot_rotation_c2[0], right_foot_rotation_c2[2]
        )
        + right_beta,
        ankle_roll=np.arcsin(-right_foot_rotation_c2[1]),
    )

    return left_leg, right_leg
