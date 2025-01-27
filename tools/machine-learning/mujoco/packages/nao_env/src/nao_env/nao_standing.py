from typing import Any, override

import numpy as np
from nao_interface.nao_interface import Nao
from nao_interface.poses import PENALIZED_POSE
from numpy.typing import NDArray
from rewards import (
    ConstantReward,
    HeadOverTorsoPenalty,
    RewardComposer,
    RewardContext,
    TorqueChangeRatePenalty,
)

from .nao_base_env import NaoBaseEnv

OFFSET_QPOS = np.array(
    [
        0.0,
        0.0,
        0.09,
        -0.06,
        0.01,
        -0.002,
        0.0,
        0.09,
        -0.06,
        0.01,
        0.002,
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

HEAD_SET_HEIGHT = 0.51


class NaoStanding(NaoBaseEnv):
    def __init__(
        self,
        *,
        throw_tomatoes: bool,
        **kwargs: Any,
    ) -> None:
        super().__init__(
            throw_tomatoes=throw_tomatoes,
            **kwargs,
        )

        self.current_step = 0
        self.termination_penalty = 10.0

        self.reward = (
            RewardComposer()
            .add(0.05, ConstantReward())
            .add(-0.01, TorqueChangeRatePenalty(self.model.nu, self.dt))
            .add(1.0, HeadOverTorsoPenalty())
        )

    @override
    def step(self, action: NDArray[np.floating]) -> tuple:
        self.current_step += 1
        nao = Nao(self.model, self.data)

        if self.throw_tomatoes and self.projectile.has_ground_contact():
            target = self.data.site("Robot").xpos
            alpha = self.current_step / 2500
            time_to_reach = 0.2 * (1 - alpha) + 0.1 * alpha
            self.projectile.random_throw(
                target,
                time_to_reach=time_to_reach,
                distance=1.0,
            )

        self.do_simulation(action + OFFSET_QPOS, self.frame_skip)
        head_center_z = self.data.site("head_center").xpos[2]

        if self.render_mode == "human":
            self.render()

        terminated = head_center_z < 0.3

        distinct_rewards = self.reward.rewards(RewardContext(nao, action))
        reward = sum(distinct_rewards.values())

        if terminated:
            reward -= self.termination_penalty

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
        self.set_state(
            self.init_qpos,
            self.init_qvel,
        )
        nao = Nao(self.model, self.data)
        nao.reset(PENALIZED_POSE)
        return self._get_obs()
