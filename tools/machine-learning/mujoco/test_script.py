import gymnasium as gym
from stable_baselines3 import PPO

# print("Available environments:")
# print(gym.pprint_registry(), "\n")


gym.register(
    id="NaoStandup-v1",
    entry_point="nao_standup:NaoStandup",
    max_episode_steps=1000,
)

env = gym.make("NaoStandup-v1", render_mode="human")

# env = gym.make("CartPole-v1", render_mode="human")
# env = gym.make("HumanoidStandup-v4", render_mode="human")

print(env.action_space)


# model = PPO("MlpPolicy", env, verbose=1)
# model.learn(total_timesteps=10_000)
#
# vec_env = model.get_env()
#
#
# obs = vec_env.reset()
# for i in range(1000000000):
#     action, _states = model.predict(obs, deterministic=True)
#     obs, reward, done, info = vec_env.step(action)
#     vec_env.render()
#
#     # VecEnv resets automatically
#     if done:
#         obs = vec_env.reset()
#
env.close()
