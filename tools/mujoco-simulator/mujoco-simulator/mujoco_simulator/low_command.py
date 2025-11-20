import numpy as np
from mujoco import MjData, MjModel
from mujoco_rust_server.booster_types import LowCommand

from mujoco_simulator.joint_actuator_info import JointActuatorInfo


class RobotPositionControl:
    def __init__(
        self, model: MjModel, actuator_info: list[JointActuatorInfo]
    ) -> None:
        self.model = model
        self.qpos_indices = np.array([info.qpos_addr for info in actuator_info])
        self.qvel_indices = np.array([info.qvel_addr for info in actuator_info])
        self.actuator_indices = np.array(
            [info.qfrc_actuator_addr for info in actuator_info]
        )

    def apply_control(
        self, data: MjData, low_command: LowCommand | None
    ) -> None:
        if low_command is None:
            return

        q = np.array([cmd.position for cmd in low_command.motor_commands])
        dq = np.array([cmd.velocity for cmd in low_command.motor_commands])
        tau = np.array([cmd.torque for cmd in low_command.motor_commands])
        kp = np.array([cmd.kp for cmd in low_command.motor_commands])
        kd = np.array([cmd.kd for cmd in low_command.motor_commands])
        weight = np.array([cmd.weight for cmd in low_command.motor_commands])

        current_position = data.qpos[self.qpos_indices]
        current_velocity = data.qvel[self.qvel_indices]

        # TODO(oleflb): booster supposedly clips position to joint limits first
        control_torque = np.clip(
            kp * (q - current_position) + kd * (dq - current_velocity) + tau,
            self.model.actuator_ctrlrange[:, 0],
            self.model.actuator_ctrlrange[:, 1],
        )
        smoothed_control_torque = (
            weight * control_torque
            + (1 - weight) * data.ctrl[self.actuator_indices]
        )
        data.ctrl[self.actuator_indices] = smoothed_control_torque
