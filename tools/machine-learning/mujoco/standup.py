import gymnasium as gym

from stable_baselines3 import PPO

env = gym.make("CartPole-v1", render_mode="rgb_array")
env = gym.wrappers.RecordVideo(env=env, video_folder="recordings", name_prefix="test-video", episode_trigger=lambda x: x % 10 == 0)


model = PPO("MlpPolicy", env, verbose=1)
model.learn(total_timesteps=10_000)

vec_env = model.get_env()

env.start_video_recorder()
if vec_env is None:
    raise ValueError("Model does not have a VecEnv")

obs = vec_env.reset()
for i in range(1000):
    action, _states = model.predict(obs, deterministic=True)
    obs, reward, done, info = vec_env.step(action)
    vec_env.render()
    # VecEnv resets automatically
    if done:
      obs = env.reset()

env.close_video_recorder()
env.close()
