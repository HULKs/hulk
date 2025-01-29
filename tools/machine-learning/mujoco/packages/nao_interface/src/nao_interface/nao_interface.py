from collections.abc import Callable

import mujoco
import numpy as np
from numpy.typing import NDArray


class HeadJoints:
    def __init__(
        self,
        getter: Callable[[str], np.floating],
        setter: Callable[[str], np.floating],
    ) -> None:
        self.getter = getter
        self.setter = setter

    def from_dict(self, values: dict) -> None:
        for k, v in values.items():
            self.setter(k, v)

    @property
    def yaw(self) -> np.floating:
        return self.getter("yaw")

    @yaw.setter
    def yaw(self, value: np.floating) -> None:
        self.setter("yaw", value)

    @property
    def pitch(self) -> np.floating:
        return self.getter("pitch")

    @pitch.setter
    def pitch(self, value: np.floating) -> None:
        self.setter("pitch", value)


class LegJoints:
    def __init__(
        self,
        getter: Callable[[str], np.floating],
        setter: Callable[[str], np.floating],
    ) -> None:
        self.getter = getter
        self.setter = setter

    def from_dict(self, values: dict) -> None:
        for k, v in values.items():
            self.setter(k, v)

    @property
    def ankle_pitch(self) -> np.floating:
        return self.getter("ankle_pitch")

    @ankle_pitch.setter
    def ankle_pitch(self, value: np.floating) -> None:
        self.setter("ankle_pitch", value)

    @property
    def ankle_roll(self) -> np.floating:
        return self.getter("ankle_roll")

    @ankle_roll.setter
    def ankle_roll(self, value: np.floating) -> None:
        self.setter("ankle_roll", value)

    @property
    def hip_pitch(self) -> np.floating:
        return self.getter("hip_pitch")

    @hip_pitch.setter
    def hip_pitch(self, value: np.floating) -> None:
        self.setter("hip_pitch", value)

    @property
    def hip_roll(self) -> np.floating:
        return self.getter("hip_roll")

    @hip_roll.setter
    def hip_roll(self, value: np.floating) -> None:
        self.setter("hip_roll", value)

    @property
    def hip_yaw_pitch(self) -> np.floating:
        return self.getter("hip_yaw_pitch")

    @hip_yaw_pitch.setter
    def hip_yaw_pitch(self, value: np.floating) -> None:
        self.setter("hip_yaw_pitch", value)

    @property
    def knee_pitch(self) -> np.floating:
        return self.getter("knee_pitch")

    @knee_pitch.setter
    def knee_pitch(self, value: np.floating) -> None:
        self.setter("knee_pitch", value)


class ArmJoints:
    def __init__(
        self,
        getter: Callable[[str], np.floating],
        setter: Callable[[str], np.floating],
    ) -> None:
        self.getter = getter
        self.setter = setter

    def from_dict(self, values: dict) -> None:
        for k, v in values.items():
            # TODO: remove once hands are implemented
            if k != "hand":
                self.setter(k, v)

    @property
    def elbow_roll(self) -> np.floating:
        return self.getter("elbow_roll")

    @elbow_roll.setter
    def elbow_roll(self, value: np.floating) -> None:
        self.setter("elbow_roll", value)

    @property
    def elbow_yaw(self) -> np.floating:
        return self.getter("elbow_yaw")

    @elbow_yaw.setter
    def elbow_yaw(self, value: np.floating) -> None:
        self.setter("elbow_yaw", value)

    @property
    def shoulder_pitch(self) -> np.floating:
        return self.getter("shoulder_pitch")

    @shoulder_pitch.setter
    def shoulder_pitch(self, value: np.floating) -> None:
        self.setter("shoulder_pitch", value)

    @property
    def shoulder_roll(self) -> np.floating:
        return self.getter("shoulder_roll")

    @shoulder_roll.setter
    def shoulder_roll(self, value: np.floating) -> None:
        self.setter("shoulder_roll", value)

    @property
    def wrist_yaw(self) -> np.floating:
        return self.getter("wrist_yaw")

    @wrist_yaw.setter
    def wrist_yaw(self, value: np.floating) -> None:
        self.setter("wrist_yaw", value)


class NaoJoints:
    def __init__(
        self,
        getter: Callable[[str], np.floating],
        setter: Callable[[str], np.floating],
    ) -> None:
        self.getter = getter
        self.setter = setter
        self.head = HeadJoints(
            lambda joint_name: getter(f"head.{joint_name}"),
            lambda joint_name, value: setter(f"head.{joint_name}", value),
        )
        self.left_leg = LegJoints(
            lambda joint_name: getter(f"left_leg.{joint_name}"),
            lambda joint_name, value: setter(f"left_leg.{joint_name}", value),
        )
        self.right_leg = LegJoints(
            lambda joint_name: getter(f"right_leg.{joint_name}"),
            lambda joint_name, value: setter(f"right_leg.{joint_name}", value),
        )
        self.left_arm = ArmJoints(
            lambda joint_name: getter(f"left_arm.{joint_name}"),
            lambda joint_name, value: setter(f"left_arm.{joint_name}", value),
        )
        self.right_arm = ArmJoints(
            lambda joint_name: getter(f"right_arm.{joint_name}"),
            lambda joint_name, value: setter(f"right_arm.{joint_name}", value),
        )

    def from_dict(self, values: dict) -> None:
        for k, v in values.items():
            match k:
                case "head":
                    self.head.from_dict(v)
                case "left_arm":
                    self.left_arm.from_dict(v)
                case "left_leg":
                    self.left_leg.from_dict(v)
                case "right_arm":
                    self.right_arm.from_dict(v)
                case "right_leg":
                    self.right_leg.from_dict(v)


class Nao:
    def __init__(
        self,
        model: mujoco.MjModel,
        data: mujoco.MjData,
        fsr_scale: float = 1.0,
    ) -> None:
        self.model = model
        self.data = data
        self.actuators = NaoJoints(
            lambda joint_name: self.data.actuator(joint_name).ctrl,
            lambda joint_name, value: self.data.actuator(
                joint_name,
            ).__setattr__("ctrl", value),
        )
        self.positions = NaoJoints(
            lambda joint_name: self.data.joint(joint_name).qpos,
            lambda joint_name, value: self.data.joint(joint_name).__setattr__(
                "qpos",
                value,
            ),
        )
        self.fsr_scale = fsr_scale

    def set_transform(
        self, position: NDArray[np.floating], quaternion: NDArray[np.floating]
    ) -> None:
        nao = self.data.body("Nao")
        nao.xpos = position
        nao.xquat = quaternion

    def reset(self, positions: dict[str, dict[str, float]]) -> None:
        mujoco.mj_resetData(self.model, self.data)

        for part, joint_values in positions.items():
            joint_data = getattr(self.positions, part)
            actuator_data = getattr(self.actuators, part)
            for joint, value in joint_values.items():
                setattr(joint_data, joint, value)
                setattr(actuator_data, joint, value)

        mujoco.mj_forward(self.model, self.data)

    def left_fsr_values(self) -> NDArray[np.floating]:
        return self.fsr_scale * np.array(
            [
                self.data.sensor(
                    "force_sensitive_resistors.left.front_left",
                ).data,
                self.data.sensor(
                    "force_sensitive_resistors.left.front_right",
                ).data,
                self.data.sensor(
                    "force_sensitive_resistors.left.rear_left",
                ).data,
                self.data.sensor(
                    "force_sensitive_resistors.left.rear_right",
                ).data,
            ],
        )

    def right_fsr_values(self) -> NDArray[np.floating]:
        return self.fsr_scale * np.array(
            [
                self.data.sensor(
                    "force_sensitive_resistors.right.front_left",
                ).data,
                self.data.sensor(
                    "force_sensitive_resistors.right.front_right",
                ).data,
                self.data.sensor(
                    "force_sensitive_resistors.right.rear_left",
                ).data,
                self.data.sensor(
                    "force_sensitive_resistors.right.rear_right",
                ).data,
            ],
        )

    def gyroscope(self) -> NDArray[np.floating]:
        return self.data.sensor("gyroscope").data

    def accelerometer(self) -> NDArray[np.floating]:
        return self.data.sensor("accelerometer").data
