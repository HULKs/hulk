from mujoco import MjModel


class JointActuatorInfo:
    name: str
    qpos_addr: int
    qvel_addr: int
    qacc_addr: int
    qfrc_actuator_addr: int

    def __init__(self, name: str, model: MjModel) -> None:
        self.name = name
        self.qpos_addr = model.joint(name).qposadr
        self.qvel_addr = model.joint(name).dofadr
        self.qacc_addr = model.joint(name).dofadr
        self.qfrc_actuator_addr = model.actuator(name).id


def joint_actuator_info_list(model: MjModel) -> list[JointActuatorInfo]:
    joints = [
        "AAHead_yaw",
        "Head_pitch",
        "ALeft_Shoulder_Pitch",
        "Left_Shoulder_Roll",
        "Left_Elbow_Pitch",
        "Left_Elbow_Yaw",
        "ARight_Shoulder_Pitch",
        "Right_Shoulder_Roll",
        "Right_Elbow_Pitch",
        "Right_Elbow_Yaw",
        "Left_Hip_Pitch",
        "Left_Hip_Roll",
        "Left_Hip_Yaw",
        "Left_Knee_Pitch",
        "Left_Ankle_Pitch",
        "Left_Ankle_Roll",
        "Right_Hip_Pitch",
        "Right_Hip_Roll",
        "Right_Hip_Yaw",
        "Right_Knee_Pitch",
        "Right_Ankle_Pitch",
        "Right_Ankle_Roll",
    ]
    return [JointActuatorInfo(name, model) for name in joints]
