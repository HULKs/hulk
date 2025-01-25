from copy import deepcopy

import mujoco
import numpy as np
from numpy.typing import NDArray
from walking_engine import Side, State

from .base import BaseReward, RewardContext


def order_support_swing[T](state: State, left: T, right: T) -> tuple[T, T]:
    if state.support_side == Side.LEFT:
        return left, right
    return right, left


def swing_sole_to_target(
    left_foot_position: NDArray[np.floating],
    right_foot_position: NDArray[np.floating],
    state: State,
) -> NDArray[np.floating]:
    support_foot_position, swing_foot_position = order_support_swing(
        state,
        left_foot_position,
        right_foot_position,
    )

    support_to_swing = state.end_feet.swing_sole - state.end_feet.support_sole
    support_to_swing = np.array([support_to_swing.x, support_to_swing.y, 0])

    target_swing_position = support_foot_position + support_to_swing

    return swing_foot_position - target_swing_position


class SwingFootDestinationReward(BaseReward):
    def __init__(self, dt: float) -> None:
        self.last_state = None
        self.last_left_foot_position = np.zeros(3)
        self.last_right_foot_position = np.zeros(3)
        self.dt = dt

    def reward(self, context: RewardContext) -> np.floating:
        if (
            self.last_state is None
            or context.walk_state.support_side != self.last_state.support_side
        ):
            self.last_state = deepcopy(context.walk_state)
            return np.float32(0.0)

        last_swing_sole_to_target = swing_sole_to_target(
            self.last_left_foot_position,
            self.last_right_foot_position,
            self.last_state,
        )
        current_swing_sole_to_target = swing_sole_to_target(
            context.nao.data.site("left_sole").xpos.copy(),
            context.nao.data.site("right_sole").xpos.copy(),
            context.walk_state,
        )

        self.last_state = deepcopy(context.walk_state)
        self.last_left_foot_position[:] = context.nao.data.site(
            "left_sole"
        ).xpos
        self.last_right_foot_position[:] = context.nao.data.site(
            "right_sole"
        ).xpos

        return np.mean(
            (
                np.square(last_swing_sole_to_target / self.dt)
                - np.square(current_swing_sole_to_target / self.dt)
            ),
        )

    def reset(self) -> None:
        self.last_state = None
        self.last_left_foot_position[:] = 0.0
        self.last_right_foot_position[:] = 0.0


class ConstantSupportFootPositionPenalty(BaseReward):
    def __init__(self) -> None:
        self.last_state = None
        self.last_support_foot_position = np.zeros(3)

    def reward(self, context: RewardContext) -> np.floating:
        if (
            self.last_state is None
            or context.walk_state.support_side != self.last_state.support_side
        ):
            self.last_state = deepcopy(context.walk_state)
            return np.float32(0.0)

        current_support_foot_position, _ = order_support_swing(
            context.walk_state,
            context.nao.data.site("left_sole").xpos,
            context.nao.data.site("right_sole").xpos,
        )

        slip_distance = np.linalg.norm(
            current_support_foot_position - self.last_support_foot_position
        )

        self.last_state = deepcopy(context.walk_state)
        self.last_support_foot_position[:] = current_support_foot_position

        return slip_distance

    def reset(self) -> None:
        self.last_state = None
        self.last_support_foot_position[:] = 0.0


class ConstantSupportFootOrientationPenalty(BaseReward):
    def __init__(self) -> None:
        self.last_state = None
        self.last_support_foot_orientation = np.zeros(4)

    def reward(self, context: RewardContext) -> np.floating:
        if (
            self.last_state is None
            or context.walk_state.support_side != self.last_state.support_side
        ):
            self.last_state = deepcopy(context.walk_state)
            return np.float32(0.0)

        current_support_foot_orientation, _ = order_support_swing(
            context.walk_state,
            context.nao.data.site("left_sole").xmat,
            context.nao.data.site("right_sole").xmat,
        )

        q0 = self.last_support_foot_orientation
        q1 = np.zeros(4)
        mujoco.mju_mat2Quat(q1, current_support_foot_orientation)
        interpolant = self._interpolant(q0, q1)

        velocity = np.zeros(3)
        mujoco.mju_quat2Vel(velocity, interpolant, 1.0)

        self.last_state = deepcopy(context.walk_state)
        self.last_support_foot_orientation[:] = q1

        return np.linalg.norm(velocity)

    def _interpolant(
        self, q0: NDArray[np.floating], q1: NDArray[np.floating]
    ) -> NDArray[np.floating]:
        tmp = np.zeros(4)
        # tmp is unused
        mujoco.mju_negQuat(tmp, q0)
        # q0 is unused
        mujoco.mju_mulQuat(q0, q1, tmp)
        return q0
