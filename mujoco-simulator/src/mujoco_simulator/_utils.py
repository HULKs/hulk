import numpy as np
from mujoco import MjData, MjModel
from mujoco_rust_server.booster_types import ImuState, LowState, MotorState
from scipy.spatial.transform import Rotation


def mj_quaternion_to_rpy(q_wxyz: np.ndarray) -> np.ndarray:
    # Convert wxyz to xyzw
    q_xyzw = np.roll(q_wxyz, -1)
    rotation = Rotation.from_quat(q_xyzw)
    return rotation.as_euler("xyz", degrees=False)


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
