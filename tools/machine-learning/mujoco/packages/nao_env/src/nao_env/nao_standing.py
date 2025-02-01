from typing import Any, override

import numpy as np
from gymnasium import utils
from nao_interface.poses import READY_POSE
from numpy.typing import NDArray
from rewards import (
    ConstantReward,
    ControlAmplitudePenalty,
    HeadOverTorsoPenalty,
    RewardComposer,
    RewardContext,
    TorqueChangeRatePenalty,
)

from .nao_base_env import NaoBaseEnv

OFFSET_QPOS = np.array(
    [
        0.0,
        0.010821502783627257,
        -0.3107718500421241,
        0.8246279211008891,
        -0.513856071058765,
        -0.010821502783627453,
        -0.010821502783627257,
        -0.3107718500421241,
        0.8246279211008891,
        -0.513856071058765,
        0.010821502783627453,
        1.57,
        0.1,
        -1.57,
        0.0,
        0.0,
        1.57,
        -0.1,
        1.57,
        0.0,
        0.0,
    ],
)

HEAD_SET_HEIGHT = 0.493


class NaoStanding(NaoBaseEnv, utils.EzPickle):
    def __init__(
        self,
        *,
        throw_tomatoes: bool,
        **kwargs: Any,
    ) -> None:
        super().__init__(
            throw_tomatoes=throw_tomatoes,
            sensor_delay=3,
            **kwargs,
        )

        self.current_step = 0
        self.next_throw_at = 500
        self.expected_number_of_frames_between_throws = 120
        self.rng = np.random.default_rng()

        self.reward = (
            RewardComposer()
            .add(0.02, ConstantReward())
            .add(-0.001, TorqueChangeRatePenalty(self.model.nu, self.dt))
            .add(-0.001, ControlAmplitudePenalty())
            .add(-0.5, HeadOverTorsoPenalty())
        )
        utils.EzPickle.__init__(self, **kwargs)

    def _should_throw_tomato(self) -> bool:
        allowed_to_throw = (
            self.current_step >= self.next_throw_at
            and self.projectile.has_ground_contact()
        )
        if allowed_to_throw:
            self.next_throw_at = self.current_step + self.rng.poisson(
                self.expected_number_of_frames_between_throws
            )

        return allowed_to_throw

    @override
    def step(self, action: NDArray[np.floating]) -> tuple:
        self.current_step += 1

        if self.throw_tomatoes and self._should_throw_tomato():
            target = self.data.site("Robot").xpos
            time_to_reach = 0.15
            self.projectile.random_throw(
                target,
                time_to_reach=time_to_reach,
                distance=1.0,
            )

        self.do_simulation(action + OFFSET_QPOS, self.frame_skip)

        if self.render_mode == "human":
            self.render()

        distinct_rewards = self.reward.rewards(RewardContext(self.nao, action))
        reward = sum(distinct_rewards.values())

        terminated = False
        self.current_step += 1
        return (
            self._get_obs(),
            reward,
            terminated,
            False,
            distinct_rewards,
        )

    @override
    def reset_model(self) -> NDArray[np.floating]:
        self.current_step = 0
        self.next_throw_at = 500
        self.reward.reset()
        self.set_state(
            self.init_qpos,
            self.init_qvel,
        )
        self.nao.reset(READY_POSE)
        return self._get_obs()
