import os

import gymnasium as gym
from stable_baselines3 import PPO

RENDER_MODE = "rgb_array"
USE_GPU = True

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
    max_episode_steps=1000,
)

env = gym.make("NaoStandup-v1", render_mode=RENDER_MODE)

if RENDER_MODE == "rgb_array":
    env = gym.wrappers.RecordVideo(
        env=env,
        video_folder="recordings",
        name_prefix="test-video",
    )

# env.disable_logger = True

model = PPO("MlpPolicy", env, verbose=1)
model.learn(total_timesteps=10_000)

vec_env = model.get_env()

if RENDER_MODE == "rgb_array":
    env.start_video_recorder()

obs = vec_env.reset()
for i in range(1000):
    action, _states = model.predict(obs, deterministic=True)
    obs, reward, done, info = vec_env.step(action)
    vec_env.render()

    # VecEnv resets automatically
    if done:
        obs = vec_env.reset()

if RENDER_MODE == "rgb_array":
    env.close_video_recorder()

env.close()
