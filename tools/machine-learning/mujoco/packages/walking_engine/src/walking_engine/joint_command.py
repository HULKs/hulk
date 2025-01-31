import numpy as np
from kinematics import LowerBodyJoints, foot_to_isometry
from kinematics.inverse_kinematics import ArmJoints, leg_angles
from robot_dimensions import ANKLE_TO_SOLE
from transforms import Pose2
from transforms.transforms import isometry_from_translation

WALK_TO_ROBOT = isometry_from_translation(np.array([-0.02, 0.0, -0.23]))


def compute_lower_body_joints(
    left_sole: Pose2,
    right_sole: Pose2,
    left_lift: float,
    right_lift: float,
) -> LowerBodyJoints:
    left_foot_in_walk = isometry_from_translation(
        -ANKLE_TO_SOLE,
    ) @ foot_to_isometry(left_sole, left_lift)
    right_foot_in_walk = isometry_from_translation(
        -ANKLE_TO_SOLE,
    ) @ foot_to_isometry(right_sole, right_lift)

    return leg_angles(
        WALK_TO_ROBOT @ left_foot_in_walk,
        WALK_TO_ROBOT @ right_foot_in_walk,
    )


def compute_arm_joints(
    left_sole: Pose2,
    right_sole: Pose2,
    *,
    pitch_factor: float = 8.0,
) -> tuple[ArmJoints, ArmJoints]:
    left_arm = ArmJoints()
    left_arm.shoulder_pitch = -pitch_factor * right_sole.x

    right_arm = ArmJoints()
    right_arm.shoulder_pitch = -pitch_factor * left_sole.x

    return left_arm, right_arm
