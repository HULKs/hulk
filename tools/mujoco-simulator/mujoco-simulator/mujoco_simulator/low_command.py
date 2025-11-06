import numpy as np
from mujoco import MjData, MjModel
from mujoco_rust_server.booster_types import LowCommand


def get_control_input(
    model: MjModel, data: MjData, current_low_command: LowCommand | None
) -> np.ndarray:
    if current_low_command is None:
        return np.zeros(model.nu)

    q = np.array([cmd.position for cmd in current_low_command.motor_commands])
    dq = np.array([cmd.velocity for cmd in current_low_command.motor_commands])
    tau = np.array([cmd.torque for cmd in current_low_command.motor_commands])
    kp = np.array([cmd.kp for cmd in current_low_command.motor_commands])
    kd = np.array([cmd.kd for cmd in current_low_command.motor_commands])
    weight = np.array(
        [cmd.weight for cmd in current_low_command.motor_commands]
    )

    current_position = data.qpos[7:]
    current_velocity = data.qvel[6:]

    # TODO(oleflb): booster clips position to joint limits first
    control_torque = np.clip(
        kp * (q - current_position) + kd * (dq - current_velocity) + tau,
        model.actuator_ctrlrange[:, 0],
        model.actuator_ctrlrange[:, 1],
    )

    return weight * control_torque + (1 - weight) * data.ctrl
