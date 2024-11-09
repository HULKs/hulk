import numpy as np

from inverse_kinematics import leg_angles, LegJoints
from transforms import isometry_from_translation, isometry_from_euler

def test_inverse_kinematics() -> None:
    left_foot = isometry_from_translation(np.array([0.0, 0.0, 0.0])) @ isometry_from_euler(0.0, 0.0, 0.0)
    right_foot = isometry_from_translation(np.array([0.0, 0.0, 0.0])) @ isometry_from_euler(0.0, 0.0, 0.0)

    left_leg, right_leg = leg_angles(left_foot, right_foot)

    print(left_leg, right_leg)
    assert False
