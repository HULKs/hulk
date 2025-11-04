from mujoco import MjData, MjModel
from mujoco_rust_server.booster_types import ImuState, LowState, MotorState

from mujoco_simulator._utils import mj_quaternion_to_rpy


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


def generate_low_state(
    data: MjData, actuator_info: list[JointActuatorInfo]
) -> LowState:
    orientation = mj_quaternion_to_rpy(data.sensor("orientation").data)
    gyro = data.sensor("angular-velocity").data
    acceleration = data.sensor("accelerometer").data

    return LowState(
        imu_state=ImuState(
            roll_pitch_yaw=orientation.tolist(),
            angular_velocity=gyro.tolist(),
            linear_acceleration=acceleration.tolist(),
        ),
        motor_state_serial=[
            MotorState(
                position=data.qpos[info.qpos_addr].item(),
                velocity=data.qvel[info.qvel_addr].item(),
                acceleration=data.qacc[info.qacc_addr].item(),
                torque=data.qfrc_actuator[info.qfrc_actuator_addr].item(),
            )
            for info in actuator_info
        ],
        motor_state_parallel=[],
    )
