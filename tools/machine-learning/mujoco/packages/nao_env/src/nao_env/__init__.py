from gymnasium.envs.mujoco.mujoco_env import (  # noqa: F401
    MujocoEnv,
)

from nao_env.nao_standing import NaoStanding

# ^^^^^ so that user gets the correct error
# message if mujoco is not installed correctly
from nao_env.nao_standup import NaoStandup

__all__ = ["NaoStanding", "NaoStandup"]
