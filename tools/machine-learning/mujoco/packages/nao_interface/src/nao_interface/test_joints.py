import numpy as np

from .joints import (
    ArmJoints,
    ArmJointsView,
    HeadJoints,
    HeadJointsView,
    Joints,
    JointsView,
    LegJoints,
    LegJointsView,
)


def test_head_joints_iteration() -> None:
    joints = HeadJoints(yaw=1, pitch=2)
    assert list(joints) == [1, 2]


def test_head_joints_items() -> None:
    joints = HeadJoints(yaw=3, pitch=4)
    assert list(joints.items()) == [("yaw", 3), ("pitch", 4)]


def test_arm_joints_iteration() -> None:
    arm = ArmJoints(
        shoulder_pitch=1,
        shoulder_roll=2,
        elbow_yaw=3,
        elbow_roll=4,
        wrist_yaw=5,
    )
    assert list(arm) == [1, 2, 3, 4, 5]


def test_arm_joints_items() -> None:
    arm = ArmJoints(
        shoulder_pitch=1,
        shoulder_roll=2,
        elbow_yaw=3,
        elbow_roll=4,
        wrist_yaw=5,
    )
    expected = [
        ("shoulder_pitch", 1),
        ("shoulder_roll", 2),
        ("elbow_yaw", 3),
        ("elbow_roll", 4),
        ("wrist_yaw", 5),
    ]
    assert list(arm.items()) == expected


def test_leg_joints_iteration() -> None:
    leg = LegJoints(
        hip_yaw_pitch=1,
        hip_roll=2,
        hip_pitch=3,
        knee_pitch=4,
        ankle_pitch=5,
        ankle_roll=6,
    )
    assert list(leg) == [1, 2, 3, 4, 5, 6]


def test_leg_joints_items() -> None:
    leg = LegJoints(
        hip_yaw_pitch=1,
        hip_roll=2,
        hip_pitch=3,
        knee_pitch=4,
        ankle_pitch=5,
        ankle_roll=6,
    )
    expected = [
        ("hip_yaw_pitch", 1),
        ("hip_roll", 2),
        ("hip_pitch", 3),
        ("knee_pitch", 4),
        ("ankle_pitch", 5),
        ("ankle_roll", 6),
    ]
    assert list(leg.items()) == expected


def test_joints_iteration_order() -> None:
    head = HeadJoints(yaw=1, pitch=2)
    left_arm = ArmJoints(3, 4, 5, 6, 7)
    left_leg = LegJoints(8, 9, 10, 11, 12, 13)
    right_arm = ArmJoints(14, 15, 16, 17, 18)
    right_leg = LegJoints(19, 20, 21, 22, 23, 24)
    joints = Joints(head, left_arm, left_leg, right_arm, right_leg)
    expected = [
        *[1, 2],
        *[8, 9, 10, 11, 12, 13],
        *[19, 20, 21, 22, 23, 24],
        *[3, 4, 5, 6, 7],
        *[14, 15, 16, 17, 18],
    ]
    assert list(joints) == expected


def test_joints_flattened_items() -> None:
    head = HeadJoints(yaw=5, pitch=10)
    left_arm = ArmJoints(1, 2, 3, 4, 5)
    left_leg = LegJoints(6, 7, 8, 9, 10, 11)
    right_arm = ArmJoints(12, 13, 14, 15, 16)
    right_leg = LegJoints(17, 18, 19, 20, 21, 22)
    joints = Joints(head, left_arm, left_leg, right_arm, right_leg)
    flattened = list(joints.flattened_items())
    expected = [
        ("head.yaw", 5),
        ("head.pitch", 10),
        ("left_leg.hip_yaw_pitch", 6),
        ("left_leg.hip_roll", 7),
        ("left_leg.hip_pitch", 8),
        ("left_leg.knee_pitch", 9),
        ("left_leg.ankle_pitch", 10),
        ("left_leg.ankle_roll", 11),
        ("right_leg.hip_yaw_pitch", 17),
        ("right_leg.hip_roll", 18),
        ("right_leg.hip_pitch", 19),
        ("right_leg.knee_pitch", 20),
        ("right_leg.ankle_pitch", 21),
        ("right_leg.ankle_roll", 22),
        ("left_arm.shoulder_pitch", 1),
        ("left_arm.shoulder_roll", 2),
        ("left_arm.elbow_yaw", 3),
        ("left_arm.elbow_roll", 4),
        ("left_arm.wrist_yaw", 5),
        ("right_arm.shoulder_pitch", 12),
        ("right_arm.shoulder_roll", 13),
        ("right_arm.elbow_yaw", 14),
        ("right_arm.elbow_roll", 15),
        ("right_arm.wrist_yaw", 16),
    ]
    assert flattened == expected


def test_head_joints_view_properties() -> None:
    storage = {}
    view = HeadJointsView(
        lambda n: storage.get(n), lambda n, v: storage.update({n: v})
    )
    view.yaw = 5
    assert storage == {"yaw": 5}
    view.pitch = 10
    assert storage == {"yaw": 5, "pitch": 10}
    storage.update({"yaw": 20, "pitch": 30})
    assert view.yaw == 20
    assert view.pitch == 30


def test_head_joints_view_set_from_joints() -> None:
    storage = {}
    view = HeadJointsView(
        lambda n: storage.get(n), lambda n, v: storage.update({n: v})
    )
    joints = HeadJoints(yaw=10, pitch=20)
    view.set_from_joints(joints)
    assert storage == {"yaw": 10, "pitch": 20}


def test_head_joints_view_set_from_dict() -> None:
    storage = {}
    view = HeadJointsView(
        lambda n: storage.get(n), lambda n, v: storage.update({n: v})
    )
    view.set_from_dict({"yaw": 5, "pitch": 6})
    assert storage == {"yaw": 5, "pitch": 6}


def test_arm_joints_view_set_from_dict_ignores_hand() -> None:
    storage = {}
    view = ArmJointsView(lambda _: None, lambda n, v: storage.update({n: v}))
    view.set_from_dict({"shoulder_pitch": 1, "hand": 2})
    assert storage == {"shoulder_pitch": 1}


def test_leg_joints_view_properties() -> None:
    storage = {}
    view = LegJointsView(
        lambda n: storage.get(n), lambda n, v: storage.update({n: v})
    )
    view.hip_yaw_pitch = 1
    view.hip_roll = 2
    view.hip_pitch = 3
    view.knee_pitch = 4
    view.ankle_pitch = 5
    view.ankle_roll = 6
    assert storage == {
        "hip_yaw_pitch": 1,
        "hip_roll": 2,
        "hip_pitch": 3,
        "knee_pitch": 4,
        "ankle_pitch": 5,
        "ankle_roll": 6,
    }
    storage.update(
        {
            "hip_yaw_pitch": 10,
            "hip_roll": 20,
            "hip_pitch": 30,
            "knee_pitch": 40,
            "ankle_pitch": 50,
            "ankle_roll": 60,
        }
    )
    assert view.hip_yaw_pitch == 10
    assert view.hip_roll == 20
    assert view.hip_pitch == 30
    assert view.knee_pitch == 40
    assert view.ankle_pitch == 50
    assert view.ankle_roll == 60


def test_joints_view_set_from_joints() -> None:
    storage = {}
    joints_view = JointsView(
        lambda n: storage.get(n), lambda n, v: storage.update({n: v})
    )
    head = HeadJoints(yaw=1, pitch=2)
    left_arm = ArmJoints(3, 4, 5, 6, 7)
    left_leg = LegJoints(8, 9, 10, 11, 12, 13)
    right_arm = ArmJoints(14, 15, 16, 17, 18)
    right_leg = LegJoints(19, 20, 21, 22, 23, 24)
    joints = Joints(head, left_arm, left_leg, right_arm, right_leg)
    joints_view.set_from_joints(joints)
    assert storage["head.yaw"] == 1
    assert storage["head.pitch"] == 2
    assert storage["left_arm.shoulder_pitch"] == 3
    assert storage["right_leg.hip_yaw_pitch"] == 19
    assert storage["right_arm.wrist_yaw"] == 18


def test_joints_view_set_from_dict() -> None:
    storage = {}
    joints_view = JointsView(
        lambda n: storage.get(n), lambda n, v: storage.update({n: v})
    )
    values = {
        "head": {"yaw": 5, "pitch": 6},
        "left_arm": {"shoulder_pitch": 7, "hand": 8},
        "right_leg": {"ankle_pitch": 9},
    }
    joints_view.set_from_dict(values)
    assert storage["head.yaw"] == 5
    assert storage["head.pitch"] == 6
    assert storage["left_arm.shoulder_pitch"] == 7
    assert "left_arm.hand" not in storage
    assert storage["right_leg.ankle_pitch"] == 9


def test_joints_view_to_numpy_default_names() -> None:
    default_names = [
        "head.yaw",
        "head.pitch",
        "left_leg.hip_yaw_pitch",
        "left_leg.hip_roll",
        "left_leg.hip_pitch",
        "left_leg.knee_pitch",
        "left_leg.ankle_pitch",
        "left_leg.ankle_roll",
        "right_leg.hip_roll",
        "right_leg.hip_pitch",
        "right_leg.knee_pitch",
        "right_leg.ankle_pitch",
        "right_leg.ankle_roll",
        "left_arm.shoulder_pitch",
        "left_arm.shoulder_roll",
        "left_arm.elbow_yaw",
        "left_arm.elbow_roll",
        "left_arm.wrist_yaw",
        "right_arm.shoulder_pitch",
        "right_arm.shoulder_roll",
        "right_arm.elbow_yaw",
        "right_arm.elbow_roll",
        "right_arm.wrist_yaw",
    ]
    storage = {name: idx + 1 for idx, name in enumerate(default_names)}
    joints_view = JointsView(lambda n: storage.get(n), lambda _n, _v: None)
    arr = joints_view.to_numpy()
    expected = np.array(
        [idx + 1 for idx in range(len(default_names))], dtype=np.float64
    )
    assert np.array_equal(arr, expected)


def test_joints_view_to_numpy_custom_names() -> None:
    storage = {
        "head.yaw": 10,
        "left_arm.shoulder_pitch": 20,
        "right_leg.ankle_roll": 30,
    }
    joints_view = JointsView(lambda n: storage.get(n), lambda _n, _v: None)
    names = ["head.yaw", "right_leg.ankle_roll", "left_arm.shoulder_pitch"]
    arr = joints_view.to_numpy(names)
    assert np.array_equal(arr, np.array([10, 30, 20]))


def test_joints_view_default_names_omit_right_leg_hip_yaw_pitch() -> None:
    default_names = [
        "head.yaw",
        "head.pitch",
        "left_leg.hip_yaw_pitch",
        "left_leg.hip_roll",
        "left_leg.hip_pitch",
        "left_leg.knee_pitch",
        "left_leg.ankle_pitch",
        "left_leg.ankle_roll",
        "right_leg.hip_roll",
        "right_leg.hip_pitch",
        "right_leg.knee_pitch",
        "right_leg.ankle_pitch",
        "right_leg.ankle_roll",
        "left_arm.shoulder_pitch",
        "left_arm.shoulder_roll",
        "left_arm.elbow_yaw",
        "left_arm.elbow_roll",
        "left_arm.wrist_yaw",
        "right_arm.shoulder_pitch",
        "right_arm.shoulder_roll",
        "right_arm.elbow_yaw",
        "right_arm.elbow_roll",
        "right_arm.wrist_yaw",
    ]
    assert "right_leg.hip_yaw_pitch" not in default_names
