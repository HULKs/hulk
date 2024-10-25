from gymnasium.envs.mujoco.mujoco_env import MujocoEnv, MuJocoPyEnv  # isort:skip

# ^^^^^ so that user gets the correct error
# message if mujoco is not installed correctly

from nao_standup.nao_standup_env import NaoStandup
