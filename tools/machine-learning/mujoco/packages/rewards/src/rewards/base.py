from dataclasses import dataclass

import numpy as np
from nao_interface import Nao
from numpy.typing import NDArray
from walking_engine import State


@dataclass
class RewardContext:
    nao: Nao
    walk_state: State
    action: NDArray[np.floating]


class BaseReward:
    def reward(self, context: RewardContext) -> np.floating:
        _ = context
        raise NotImplementedError()

    def reset(self) -> None:
        pass

    def name(self) -> str:
        return self.__class__.__name__
