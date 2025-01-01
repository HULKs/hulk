from typing import Callable
import gymnasium as gym

import wandb


class WandbVideoLoggerWrapperEnv(gym.Env):
    def __init__(self, env: gym.Env, video_trigger: Callable[[int], bool]):
        self.env = env
        self.video_trigger = video_trigger
        self.episode_id = 0

    def reset(self):
        self.episode_id += 1
        return self.env.reset()

    def step(self, action):
        obs, reward, done, info = self.env.step(action)
        if self.video_trigger(self.episode_id):
            self.run.log_video(self.env.render("rgb_array"))
        if done:
            self.episode_id += 1
        return obs, reward, done, info

    def render(self, mode="human"):
        return self.env.render(mode)

    def close(self):
        return self.env.close()
