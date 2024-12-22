from kinematics import LegJoints
from kinematics import foot_to_isometry
from kinematics.inverse_kinematics import leg_angles
import numpy as np
from transforms import Pose2
from transforms.transforms import isometry_from_translation

from robot_dimensions import ANKLE_TO_SOLE


def joint_command(
    left_sole: Pose2,
    right_sole: Pose2,
    left_lift: float,
    right_lift: float,
) -> LegJoints:
    walk_to_robot = isometry_from_translation(np.array([-0.02, 0.0, -0.23]))

    left_foot_in_walk = isometry_from_translation(
        -ANKLE_TO_SOLE
    ) @ foot_to_isometry(left_sole, left_lift)
    right_foot_in_walk = isometry_from_translation(
        -ANKLE_TO_SOLE
    ) @ foot_to_isometry(right_sole, right_lift)

    left_leg_joints, right_leg_joints = leg_angles(
        walk_to_robot @ left_foot_in_walk,
        walk_to_robot @ right_foot_in_walk,
    )

    return left_leg_joints, right_leg_joints
