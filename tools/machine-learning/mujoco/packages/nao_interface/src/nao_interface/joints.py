from collections.abc import Callable, Iterator, Sequence
from dataclasses import dataclass
from itertools import chain

import numpy as np
from numpy.typing import NDArray

JOINT_NAMES = [
    "head.yaw",
    "head.pitch",
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
    "hip_yaw_pitch",
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
]


@dataclass
class HeadJoints[T]:
    yaw: T
    pitch: T

    def __iter__(self) -> Iterator[T]:
        return iter((self.yaw, self.pitch))

    def items(self) -> Iterator[tuple[str, T]]:
        return iter(
            (
                ("yaw", self.yaw),
                ("pitch", self.pitch),
            )
        )


@dataclass
class ArmJoints[T]:
    shoulder_pitch: T
    shoulder_roll: T
    elbow_yaw: T
    elbow_roll: T
    wrist_yaw: T

    def __iter__(self) -> Iterator[T]:
        return iter(
            (
                self.shoulder_pitch,
                self.shoulder_roll,
                self.elbow_yaw,
                self.elbow_roll,
                self.wrist_yaw,
            )
        )

    def items(self) -> Iterator[tuple[str, T]]:
        return iter(
            (
                ("shoulder_pitch", self.shoulder_pitch),
                ("shoulder_roll", self.shoulder_roll),
                ("elbow_yaw", self.elbow_yaw),
                ("elbow_roll", self.elbow_roll),
                ("wrist_yaw", self.wrist_yaw),
            )
        )


@dataclass
class LegJoints[T]:
    hip_roll: T
    hip_pitch: T
    knee_pitch: T
    ankle_pitch: T
    ankle_roll: T

    def __iter__(self) -> Iterator[T]:
        return iter(
            (
                self.hip_roll,
                self.hip_pitch,
                self.knee_pitch,
                self.ankle_pitch,
                self.ankle_roll,
            )
        )

    def items(self) -> Iterator[tuple[str, T]]:
        return iter(
            (
                ("hip_roll", self.hip_roll),
                ("hip_pitch", self.hip_pitch),
                ("knee_pitch", self.knee_pitch),
                ("ankle_pitch", self.ankle_pitch),
                ("ankle_roll", self.ankle_roll),
            )
        )


@dataclass
class Joints[T]:
    head: HeadJoints[T]
    left_arm: ArmJoints[T]
    right_arm: ArmJoints[T]
    hip_yaw_pitch: T
    left_leg: LegJoints[T]
    right_leg: LegJoints[T]

    def __iter__(self) -> Iterator[T]:
        return chain(
            iter(self.head),
            iter(self.left_arm),
            iter(self.right_arm),
            (self.hip_yaw_pitch,),
            iter(self.left_leg),
            iter(self.right_leg),
        )

    def flattened_items(self) -> Iterator[tuple[str, T]]:
        return chain(
            (("head." + name, value) for name, value in self.head.items()),
            (
                ("left_arm." + name, value)
                for name, value in self.left_arm.items()
            ),
            (
                ("right_arm." + name, value)
                for name, value in self.right_arm.items()
            ),
            (("hip_yaw_pitch", self.hip_yaw_pitch),),
            (
                ("left_leg." + name, value)
                for name, value in self.left_leg.items()
            ),
            (
                ("right_leg." + name, value)
                for name, value in self.right_leg.items()
            ),
        )


class HeadJointsView[T]:
    def __init__(
        self,
        getter: Callable[[str], T],
        setter: Callable[[str, T], None],
    ) -> None:
        self.getter = getter
        self.setter = setter

    def set_from_joints(self, joints: HeadJoints[T]) -> None:
        for k, v in joints.items():
            self.setter(k, v)

    def set_from_dict(self, values: dict) -> None:
        for k, v in values.items():
            self.setter(k, v)

    @property
    def yaw(self) -> T:
        return self.getter("yaw")

    @yaw.setter
    def yaw(self, value: T) -> None:
        self.setter("yaw", value)

    @property
    def pitch(self) -> T:
        return self.getter("pitch")

    @pitch.setter
    def pitch(self, value: T) -> None:
        self.setter("pitch", value)


class LegJointsView[T]:
    def __init__(
        self,
        getter: Callable[[str], T],
        setter: Callable[[str, T], None],
    ) -> None:
        self.getter = getter
        self.setter = setter

    def set_from_joints(self, joints: LegJoints[T]) -> None:
        for k, v in joints.items():
            self.setter(k, v)

    def set_from_dict(self, values: dict) -> None:
        for k, v in values.items():
            self.setter(k, v)

    @property
    def hip_roll(self) -> T:
        return self.getter("hip_roll")

    @hip_roll.setter
    def hip_roll(self, value: T) -> None:
        self.setter("hip_roll", value)

    @property
    def hip_pitch(self) -> T:
        return self.getter("hip_pitch")

    @hip_pitch.setter
    def hip_pitch(self, value: T) -> None:
        self.setter("hip_pitch", value)

    @property
    def knee_pitch(self) -> T:
        return self.getter("knee_pitch")

    @knee_pitch.setter
    def knee_pitch(self, value: T) -> None:
        self.setter("knee_pitch", value)

    @property
    def ankle_pitch(self) -> T:
        return self.getter("ankle_pitch")

    @ankle_pitch.setter
    def ankle_pitch(self, value: T) -> None:
        self.setter("ankle_pitch", value)

    @property
    def ankle_roll(self) -> T:
        return self.getter("ankle_roll")

    @ankle_roll.setter
    def ankle_roll(self, value: T) -> None:
        self.setter("ankle_roll", value)


class ArmJointsView[T]:
    def __init__(
        self,
        getter: Callable[[str], T],
        setter: Callable[[str, T], None],
    ) -> None:
        self.getter = getter
        self.setter = setter

    def set_from_joints(self, joints: ArmJoints[T]) -> None:
        for k, v in joints.items():
            self.setter(k, v)

    def set_from_dict(self, values: dict) -> None:
        for k, v in values.items():
            # TODO: remove once hands are implemented
            if k == "hand":
                continue
            self.setter(k, v)

    @property
    def elbow_roll(self) -> T:
        return self.getter("elbow_roll")

    @elbow_roll.setter
    def elbow_roll(self, value: T) -> None:
        self.setter("elbow_roll", value)

    @property
    def elbow_yaw(self) -> T:
        return self.getter("elbow_yaw")

    @elbow_yaw.setter
    def elbow_yaw(self, value: T) -> None:
        self.setter("elbow_yaw", value)

    @property
    def shoulder_pitch(self) -> T:
        return self.getter("shoulder_pitch")

    @shoulder_pitch.setter
    def shoulder_pitch(self, value: T) -> None:
        self.setter("shoulder_pitch", value)

    @property
    def shoulder_roll(self) -> T:
        return self.getter("shoulder_roll")

    @shoulder_roll.setter
    def shoulder_roll(self, value: T) -> None:
        self.setter("shoulder_roll", value)

    @property
    def wrist_yaw(self) -> T:
        return self.getter("wrist_yaw")

    @wrist_yaw.setter
    def wrist_yaw(self, value: T) -> None:
        self.setter("wrist_yaw", value)


class JointsView[T]:
    def __init__(
        self,
        getter: Callable[[str], T],
        setter: Callable[[str, T], None],
    ) -> None:
        self.getter = getter
        self.setter = setter
        self.head = HeadJointsView(
            lambda joint_name: getter(f"head.{joint_name}"),
            lambda joint_name, value: setter(f"head.{joint_name}", value),
        )
        self.left_arm = ArmJointsView(
            lambda joint_name: getter(f"left_arm.{joint_name}"),
            lambda joint_name, value: setter(f"left_arm.{joint_name}", value),
        )
        self.right_arm = ArmJointsView(
            lambda joint_name: getter(f"right_arm.{joint_name}"),
            lambda joint_name, value: setter(f"right_arm.{joint_name}", value),
        )
        self.left_leg = LegJointsView(
            lambda joint_name: getter(f"left_leg.{joint_name}"),
            lambda joint_name, value: setter(f"left_leg.{joint_name}", value),
        )
        self.right_leg = LegJointsView(
            lambda joint_name: getter(f"right_leg.{joint_name}"),
            lambda joint_name, value: setter(f"right_leg.{joint_name}", value),
        )

    @property
    def hip_yaw_pitch(self) -> T:
        return self.getter("hip_yaw_pitch")

    @hip_yaw_pitch.setter
    def hip_yaw_pitch(self, value: T) -> None:
        self.setter("hip_yaw_pitch", value)

    def set_from_joints(self, joints: Joints[T]) -> None:
        self.head.set_from_joints(joints.head)
        self.left_arm.set_from_joints(joints.left_arm)
        self.right_arm.set_from_joints(joints.right_arm)
        self.hip_yaw_pitch = joints.hip_yaw_pitch
        self.left_leg.set_from_joints(joints.left_leg)
        self.right_leg.set_from_joints(joints.right_leg)

    def set_from_dict(self, values: dict) -> None:
        for k, v in values.items():
            match k:
                case "head":
                    self.head.set_from_dict(v)
                case "left_arm":
                    self.left_arm.set_from_dict(v)
                case "right_arm":
                    self.right_arm.set_from_dict(v)
                case "hip_yaw_pitch":
                    self.hip_yaw_pitch = v
                case "left_leg":
                    self.left_leg.set_from_dict(v)
                case "right_leg":
                    self.right_leg.set_from_dict(v)

    def set_from_numpy(
        self,
        values: NDArray,
        actuator_names: Sequence[str] = JOINT_NAMES,
    ) -> None:
        for name, value in zip(actuator_names, values, strict=True):
            self.setter(name, value)

    def to_numpy(self, names: Sequence[str] = JOINT_NAMES) -> NDArray:
        return np.array([self.getter(name) for name in names])
