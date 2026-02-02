from mujoco import MjData
from mujoco_rust_server.booster_types import ImuState, LowState, MotorState

from mujoco_simulator._utils import mj_quaternion_to_rpy
from mujoco_simulator.joint_actuator_info import JointActuatorInfo


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
        motor_state_parallel=[],
        motor_state_serial=[
            MotorState(
                mode=0,
                position=data.qpos[info.qpos_addr].item(),
                velocity=data.qvel[info.qvel_addr].item(),
                acceleration=data.qacc[info.qacc_addr].item(),
                torque=data.qfrc_actuator[info.qfrc_actuator_addr].item(),
                temperature=0,
                lost=0,
                reserve=[0, 0],
            )
            for info in actuator_info
        ],
    )
