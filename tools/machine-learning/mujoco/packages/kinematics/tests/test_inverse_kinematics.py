import json
from pathlib import Path

import numpy as np
from kinematics import LegJoints, leg_angles


def test_fuzz() -> None:
    fuzzing_data1 = json.loads(
        Path(__file__).parent.joinpath("standup_back_fuzz.json").read_text(),
    )
    fuzzing_data2 = json.loads(
        Path(__file__).parent.joinpath("standup_back_fuzz.json").read_text(),
    )

    for frame in fuzzing_data1 + fuzzing_data2:
        left_foot = np.array(frame["left_foot"]).reshape(4, 4).T
        right_foot = np.array(frame["right_foot"]).reshape(4, 4).T
        left_leg = LegJoints(**frame["joints"]["left_leg"])
        right_leg = LegJoints(**frame["joints"]["right_leg"])

        lower_body_joints = leg_angles(
            left_foot,
            right_foot,
        )

        np.testing.assert_allclose(
            lower_body_joints.left.to_numpy(),
            left_leg.to_numpy(),
            atol=1e-4,
        )
        np.testing.assert_allclose(
            lower_body_joints.right.to_numpy(),
            right_leg.to_numpy(),
            atol=1e-4,
        )
