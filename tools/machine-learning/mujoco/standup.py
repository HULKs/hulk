import os
import time

import gymnasium as gym
import wandb
from stable_baselines3 import PPO
from stable_baselines3.common.monitor import Monitor
from stable_baselines3.common.vec_env import DummyVecEnv, VecVideoRecorder
from wandb.integration.sb3 import WandbCallback

RENDER_MODE = "rgb_array"
USE_GPU = False

if USE_GPU:
    NVIDIA_ICD_CONFIG_PATH = "/usr/share/glvnd/egl_vendor.d/10_nvidia.json"
    if not os.path.exists(NVIDIA_ICD_CONFIG_PATH):
        with open(NVIDIA_ICD_CONFIG_PATH, "w") as f:
            _ = f.write("""{
                                "file_format_version" : "1.0.0",
                                "ICD" : {
                                    "library_path" : "libEGL_nvidia.so.0"
                                }
                            }""")

    # Configure MuJoCo to use the EGL rendering backend (requires GPU)
    os.environ["MUJOCO_GL"] = "egl"


# taken from https://gymnasium.farama.org/main/_modules/gymnasium/wrappers/record_video/
def capped_cubic_video_schedule(episode_id: int) -> bool:
    """The default episode trigger.

    This function will trigger recordings at the episode indices 0, 1, 8, 27, ..., :math:`k^3`, ..., 729, 1000, 2000, 3000, ...

    Args:
        episode_id: The episode number

    Returns:
        If to apply a video schedule number
    """
    if episode_id < 10000:
        return int(round(episode_id ** (1.0 / 3))) ** 3 == episode_id
    else:
        return episode_id % 10000 == 0


gym.register(
    id="NaoStandup-v1",
    entry_point="nao_standup:NaoStandup",
    max_episode_steps=2500,
)

config = {
    "policy_type": "MlpPolicy",
    "total_timesteps": 1000000,
    "env_name": "NaoStandup-v1",
    "render_mode": "rgb_array",
}
run = wandb.init(
    project="nao_standup",
    config=config,
    sync_tensorboard=True,
    monitor_gym=True,
    save_code=False,
)


def make_env():
    env = gym.make(config["env_name"], render_mode=config["render_mode"])
    env = Monitor(env)  # record stats such as returns
    return env


env = DummyVecEnv([make_env])
env = VecVideoRecorder(
    env,
    f"videos/{run.id}",
    record_video_trigger=capped_cubic_video_schedule,
    video_length=200,
)
model = PPO(config["policy_type"], env, verbose=1, tensorboard_log=f"runs/{run.id}")
model.learn(
    total_timesteps=config["total_timesteps"],
    callback=WandbCallback(
        gradient_save_freq=100,
        model_save_path=f"models/{run.id}",
        verbose=2,
    ),
)
run.finish()

#
# model = PPO("MlpPolicy", env, verbose=1, tensorboard_log="./logs")
# model.learn(total_timesteps=1000000, callback=WandbCallback())
#
# vec_env = model.get_env()
# assert vec_env is not None
# "vec_env is None!"
#
# if RENDER_MODE == "rgb_array":
#     env.start_video_recorder()
#
# obs = vec_env.reset()
# for i in range(10000):
#     action, _states = model.predict(obs, deterministic=True)
#     obs, reward, done, info = vec_env.step(action)
#     vec_env.render()
#
#     # VecEnv resets automatically
#     if done:
#         obs = vec_env.reset()
#
# if RENDER_MODE == "rgb_array":
#     env.close_video_recorder()
#
# env.close()
