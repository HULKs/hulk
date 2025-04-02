import numpy as np
from numpy.typing import NDArray

from .base import BaseReward, RewardContext


class ConstantReward(BaseReward):
    def reward(self, context: RewardContext) -> np.floating:
        _ = context
        return np.float32(1.0)


class ControlAmplitudePenalty(BaseReward):
    def reward(self, context: RewardContext) -> np.floating:
        return np.square(context.action).sum()


class ExternalImpactForcesPenalty(BaseReward):
    def reward(self, context: RewardContext) -> np.floating:
        return np.square(context.nao.data.cfrc_ext).sum()


class HeadHeightReward(BaseReward):
    def reward(self, context: RewardContext) -> np.floating:
        return context.nao.data.site("head_center").xpos[2]


class HeadZErrorPenalty(BaseReward):
    def __init__(self, target: float) -> None:
        self.target = target

    def reward(self, context: RewardContext) -> np.floating:
        return np.square(
            context.nao.data.site("head_center").xpos[2] - self.target
        )


class ActionRatePenalty(BaseReward):
    def __init__(self, control_dimension: int) -> None:
        self.last_action = np.zeros(control_dimension)

    def reward(self, context: RewardContext) -> np.floating:
        action_rate = np.mean(np.square(context.action - self.last_action))
        self.last_control = np.copy(context.action)
        return action_rate


class HeadXYErrorPenalty(BaseReward):
    def __init__(self, target: NDArray[np.floating]) -> None:
        self.target = target

    def reward(self, context: RewardContext) -> np.floating:
        head_center_xy = context.nao.data.site("head_center").xpos[:2]
        return np.mean(np.square(head_center_xy - self.target))


class HeadOverTorsoPenalty(BaseReward):
    def reward(self, context: RewardContext) -> np.floating:
        robot_xy = context.nao.data.site("Robot").xpos[:2]
        head_xy = context.nao.data.site("head_center").xpos[:2]
        return np.mean(np.square(head_xy - robot_xy))


class TorqueChangeRatePenalty(BaseReward):
    def __init__(self, actuator_dimension: int, dt: float) -> None:
        self.previous_force = np.zeros(actuator_dimension)
        self.is_initialized = False
        self.dt = dt

    def reward(self, context: RewardContext) -> np.floating:
        previous_torque = (
            context.nao.model.actuator_gear[:, 0] * self.previous_force
        )
        current_torque = (
            context.nao.model.actuator_gear[:, 0]
            * context.nao.data.actuator_force
        )
        torque_change_rate = np.mean(
            np.abs(previous_torque - current_torque) / self.dt
        )
        self.previous_force = np.copy(context.nao.data.actuator_force)

        if not self.is_initialized:
            self.is_initialized = True
            return np.float32(0.0)

        return torque_change_rate

    def reset(self) -> None:
        self.is_initialized = False


class XDistanceReward(BaseReward):
    def reward(self, context: RewardContext) -> np.floating:
        return context.nao.data.site("Robot").xpos[0]


class JerkPenalty(BaseReward):
    def __init__(self, dt: float) -> None:
        self.buffer = np.zeros((4, 3))
        self._empty = np.empty((3, 3))
        self.kernel = np.array([1, -3, 3, -1]) / dt**3
        self.current_step = 0

    def reward(self, context: RewardContext) -> np.floating:
        self.current_step += 1
        self._empty = self.buffer[1:]
        self.buffer[:-1] = self._empty
        self.buffer[-1] = context.nao.data.site("Robot").xpos

        if self.current_step < 4:
            return np.float32(0.0)

        jerk = self.kernel @ self.buffer
        return np.linalg.norm(jerk)

    def reset(self) -> None:
        self.buffer[:] = 0.0
        self.current_step = 0
