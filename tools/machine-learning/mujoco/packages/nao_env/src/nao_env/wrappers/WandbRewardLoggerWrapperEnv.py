import gymnasium as gym


class WandbRewardLoggerWrapperEnv(gym.Env):
    def __init__(self, env, run):
        self.env = env
        self.run = run
        self.episode_reward = 0

    def reset(self):
        self.episode_reward = 0
        return self.env.reset()

    def step(self, action):
        obs, reward, done, info = self.env.step(action)
        self.episode_reward += reward
        if done:
            self.run.log({"episode_reward": self.episode_reward})
        return obs, reward, done, info

    def render(self, mode="human"):
        return self.env.render(mode)

    def close(self):
        return self.env.close()
