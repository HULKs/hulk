import numpy as np
from mujoco import MjData, MjModel
from mujoco_rust_server.booster_types import LowCommand
from numpy.typing import NDArray

from mujoco_simulator.joint_actuator_info import JointActuatorInfo


class RobotPositionControl:
    model: MjModel
    qpos_indices: NDArray
    qvel_indices: NDArray
    actuator_indices: NDArray

    def __init__(
        self, model: MjModel, actuator_info: list[JointActuatorInfo]
    ) -> None:
        self.model = model
        self.qpos_indices = np.array(
            [info.qpos_addr for info in actuator_info], dtype=np.uint32
        )
        self.qvel_indices = np.array(
            [info.qvel_addr for info in actuator_info], dtype=np.uint32
        )
        self.actuator_indices = np.array(
            [info.qfrc_actuator_addr for info in actuator_info], dtype=np.uint32
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
        desired = (
            kp * (q - current_position) + kd * (dq - current_velocity) + tau
        )

        ctrl_min = self.model.actuator_ctrlrange[self.actuator_indices, 0]
        ctrl_max = self.model.actuator_ctrlrange[self.actuator_indices, 1]
        control_torque = np.clip(desired, ctrl_min, ctrl_max)

        # Ensure existing control values are used as floats when smoothing
        current_ctrl = data.ctrl[self.actuator_indices]
        smoothed_control_torque = (
            weight * control_torque + (1 - weight) * current_ctrl
        )
        data.ctrl[self.actuator_indices] = smoothed_control_torque
