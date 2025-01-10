from gymnasium.envs.mujoco.mujoco_env import (  # noqa: F401
    MujocoEnv,
)

from nao_env.nao_standing import NaoStanding

# ^^^^^ so that user gets the correct error
# message if mujoco is not installed correctly
from nao_env.nao_standup import NaoStandup
from nao_env.nao_walking import NaoWalking

__all__ = ["NaoStanding", "NaoStandup", "NaoWalking"]


def register():
    import gymnasium as gym

    gym.register("NaoStanding", "NaoStanding", max_episode_steps=2500)
    gym.register("NaoStandup", "NaoStandup", max_episode_steps=2500)
    gym.register("NaoWalking", "NaoWalking", max_episode_steps=2500)
