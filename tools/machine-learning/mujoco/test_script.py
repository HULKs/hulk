import gymnasium as gym
from stable_baselines3 import PPO

DEBUG = False

gym.register(
    id="NaoStandup-v1",
    entry_point="nao_env:NaoStandup",
    max_episode_steps=2500,
)

if DEBUG:
    print("Available environments:")
    print(gym.pprint_registry(), "\n")


env = gym.make("NaoStandup-v1", render_mode="human")

# env = gym.make("CartPole-v1", render_mode="human")
# env = gym.make("HumanoidStandup-v4", render_mode="human")

if DEBUG:
    print(env.action_space)
    print(env.reward_range)
    print(env.metadata)
    print(env.observation_space)
    print(env.spec, "\n")


model = PPO("MlpPolicy", env, verbose=1)
model.learn(total_timesteps=10000)

vec_env = model.get_env()
assert vec_env is not None
"vec_env is None!"

obs = vec_env.reset()
for _ in range(1000000000):
    action, _states = model.predict(obs, deterministic=True)
    obs, reward, done, info = vec_env.step(action)
    vec_env.render()

    # VecEnv resets automatically
    if done:
        obs = vec_env.reset()

env.close()
