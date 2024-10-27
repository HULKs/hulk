import os
import time

import gymnasium as gym
from stable_baselines3 import PPO

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


gym.register(
    id="NaoStandup-v1",
    entry_point="nao_standup:NaoStandup",
    max_episode_steps=2500,
)

env = gym.make("NaoStandup-v1", render_mode=RENDER_MODE)

if RENDER_MODE == "rgb_array":
    timestr = time.strftime("%Y%m%d-%H%M%S")
    env = gym.wrappers.RecordVideo(
        env=env,
        video_folder="recordings",
        name_prefix=timestr,
    )

# env.disable_logger = True

model = PPO("MlpPolicy", env, verbose=1, tensorboard_log="./logs")
model.learn(total_timesteps=1000000)

vec_env = model.get_env()
assert vec_env is not None
"vec_env is None!"

if RENDER_MODE == "rgb_array":
    env.start_video_recorder()

obs = vec_env.reset()
for i in range(10000):
    action, _states = model.predict(obs, deterministic=True)
    obs, reward, done, info = vec_env.step(action)
    vec_env.render()

    # VecEnv resets automatically
    if done:
        obs = vec_env.reset()

if RENDER_MODE == "rgb_array":
    env.close_video_recorder()

env.close()
