from typing import Self

import numpy as np
import wandb

from .base import BaseReward, RewardContext


class RewardComposer(BaseReward):
    def __init__(self) -> None:
        self.rewards: list[tuple[float | None, BaseReward]] = []
        self.has_logged = False

    def add(self, factor: float | None, reward: BaseReward) -> Self:
        self.rewards.append((factor, reward))
        return self

    def reward(self, context: RewardContext) -> np.floating:
        return np.float32(sum(self.distinct_rewards(context).values()))

    def distinct_rewards(
        self, context: RewardContext
    ) -> dict[str, np.floating]:
        return {
            reward.name(): factor * reward.reward(context)
            for factor, reward in self.rewards
            if factor is not None
        }

    def reset(self) -> None:
        if not self.has_logged and wandb.run is not None:
            self.has_logged = True
            for factor, reward in self.rewards:
                wandb.config[reward.name()] = factor or 0.0

        for _, reward in self.rewards:
            reward.reset()
