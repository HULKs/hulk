from gymnasium.envs.mujoco.mujoco_env import MujocoEnv, MuJocoPyEnv  # noqa: F401

# ^^^^^ so that user gets the correct error
# message if mujoco is not installed correctly

from nao_env.nao_standup import NaoStandup
