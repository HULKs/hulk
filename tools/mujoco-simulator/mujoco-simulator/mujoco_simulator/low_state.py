import numpy as np
from mujoco import MjData, MjModel
from mujoco_rust_server.booster_types import ImuState, LowState, MotorState

from mujoco_simulator._utils import mj_quaternion_to_rpy


def generate_low_state(model: MjModel, data: MjData) -> LowState:
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
                position=data.qpos[joint_id].item(),
                velocity=data.qvel[joint_id].item(),
                acceleration=data.qacc[joint_id].item(),
                torque=data.qfrc_actuator[joint_id].item(),
            )
            for joint_id in np.arange(1, model.njnt)
        ],
        motor_state_parallel=[],
    )
