from __future__ import annotations


class HeadJoints:
    def __init__(self, getter, setter):
        self.getter = getter
        self.setter = setter

    @property
    def yaw(self):
        return self.getter("yaw")

    @yaw.setter
    def yaw(self, value):
        self.setter("yaw", value)

    @property
    def pitch(self):
        return self.getter("pitch")

    @pitch.setter
    def pitch(self, value):
        self.setter("pitch", value)


class LegJoints:
    def __init__(self, getter, setter):
        self.getter = getter
        self.setter = setter

    @property
    def ankle_pitch(self):
        return self.getter("ankle_pitch")

    @ankle_pitch.setter
    def ankle_pitch(self, value):
        self.setter("ankle_pitch", value)

    @property
    def ankle_roll(self):
        return self.getter("ankle_roll")

    @ankle_roll.setter
    def ankle_roll(self, value):
        self.setter("ankle_roll", value)

    @property
    def hip_pitch(self):
        return self.getter("hip_pitch")

    @hip_pitch.setter
    def hip_pitch(self, value):
        self.setter("hip_pitch", value)

    @property
    def hip_roll(self):
        return self.getter("hip_roll")

    @hip_roll.setter
    def hip_roll(self, value):
        self.setter("hip_roll", value)

    @property
    def hip_yaw_pitch(self):
        return self.getter("hip_yaw_pitch")

    @hip_yaw_pitch.setter
    def hip_yaw_pitch(self, value):
        self.setter("hip_yaw_pitch", value)

    @property
    def knee_pitch(self):
        return self.getter("knee_pitch")

    @knee_pitch.setter
    def knee_pitch(self, value):
        self.setter("knee_pitch", value)


class ArmJoints:
    def __init__(self, getter, setter):
        self.getter = getter
        self.setter = setter

    @property
    def elbow_roll(self):
        return self.getter("elbow_roll")

    @elbow_roll.setter
    def elbow_roll(self, value):
        self.setter("elbow_roll", value)

    @property
    def elbow_yaw(self):
        return self.getter("elbow_yaw")

    @elbow_yaw.setter
    def elbow_yaw(self, value):
        self.setter("elbow_yaw", value)

    @property
    def shoulder_pitch(self):
        return self.getter("shoulder_pitch")

    @shoulder_pitch.setter
    def shoulder_pitch(self, value):
        self.setter("shoulder_pitch", value)

    @property
    def shoulder_roll(self):
        return self.getter("shoulder_roll")

    @shoulder_roll.setter
    def shoulder_roll(self, value):
        self.setter("shoulder_roll", value)

    @property
    def wrist_yaw(self):
        return self.getter("wrist_yaw")

    @wrist_yaw.setter
    def wrist_yaw(self, value):
        self.setter("wrist_yaw", value)


class NaoJoints:
    def __init__(self, getter, setter):
        self.getter = getter
        self.setter = setter
        self.head = HeadJoints(
            lambda joint_name: getter(f"head_{joint_name}"),
            lambda joint_name, value: setter(f"head_{joint_name}", value),
        )
        self.left_leg = LegJoints(
            lambda joint_name: getter(f"left_{joint_name}"),
            lambda joint_name, value: setter(f"left_{joint_name}", value),
        )
        self.right_leg = LegJoints(
            lambda joint_name: getter(f"right_{joint_name}"),
            lambda joint_name, value: setter(f"right_{joint_name}", value),
        )
        self.left_arm = ArmJoints(
            lambda joint_name: getter(f"left_{joint_name}"),
            lambda joint_name, value: setter(f"left_{joint_name}", value),
        )
        self.right_arm = ArmJoints(
            lambda joint_name: getter(f"right_{joint_name}"),
            lambda joint_name, value: setter(f"right_{joint_name}", value),
        )


class Nao:
    def __init__(self, model, data):
        self.model = model
        self.data = data
        self.actuators = NaoJoints(
            lambda joint_name: self.data.actuator(joint_name).ctrl,
            lambda joint_name, value: self.data.actuator(joint_name).__setattr__("ctrl", value),
        )
        self.positions = NaoJoints(
            lambda joint_name: self.data.joint(joint_name).qpos,
            lambda joint_name, value: self.data.joint(joint_name).__setattr__("qpos", value),
        )
