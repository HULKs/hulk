import gym
from gym import spaces
from gym.utils import seeding
import numpy as np
import random
import struct
import time
from websocket import create_connection


CACHED_ENV = None
ACTION_SIZE = 2 * 26
OBSERVATION_SIZE = (3 * 26) + (2 * 8) + 2


class NAOEnvMaker:
    def __new__(cls, *args, **kwargs):
        global CACHED_ENV
        if CACHED_ENV is None:
            return NAOEnv(*args, **kwargs)
        else:
            print('\033[92m' + 'Using cached Env' + '\033[0m')
            return CACHED_ENV


class NAOEnv(gym.GoalEnv):
    def __init__(self, render_mode='none', reward_type='sparse'):
        print('\033[92m' + 'Creating new Env' + '\033[0m')
        #render = render_mode == 'human'
        self.reward_type = reward_type
        self.delay = 0.01
        self.fps = 0
        self.fps_count = 0
        self.step_ctr = 0
        self.start_time = time.time()
        self.ws = create_connection("ws://localhost:9990")
        global CACHED_ENV
        CACHED_ENV = self
        self.action_space = spaces.Box(-1., 1.,
                                       shape=(ACTION_SIZE,), dtype='float32')
        self.observation_space = spaces.Dict(dict(
            desired_goal=spaces.Box(0., 1., shape=(1,), dtype='float32'),
            achieved_goal=spaces.Box(0., 1., shape=(1,), dtype='float32'),
            observation=spaces.Box(-1., 1., shape=(OBSERVATION_SIZE,), dtype='float32'),))
        self.initial_obs = None
        self.initial_obs = self.reset()

    def seed(self, seed=None):
        self.np_random, seed = seeding.np_random(seed)
        return [seed]

    def render(self, mode='none', **kwargs):
        return

    def reset(self):
        super().reset()
        self.seed()
        action = np.array([0.0 for _ in range(ACTION_SIZE)])
        obs, r, done, info = self.step(action)
        self.step_ctr = 0
        return obs

    def compute_reward(self, achieved_goal, goal):
        if achieved_goal[0] == 1.0:
            return np.array(0.0)
        else:
            return np.array(-1.0)

    def _done(self, most_recent_stability, initial_stability):
        if most_recent_stability is None or initial_stability is None:
            return False
        else:
            if (most_recent_stability == 1.0 and initial_stability == 0.0):
                print("Robot stood up")
                return True
            if (most_recent_stability == 0.0 and initial_stability != 0.0):
                print("Robot fell")
                return True
            return False

    def step(self, action):
        # send action
        action = [0.02 * (-0.5 + random.random()) for _ in range(ACTION_SIZE)]
        action_bin = struct.pack('%sf' % len(action), *action)
        self.ws.send_binary(action_bin)

        # receive observation
        observation_bin = self.ws.recv()
        obs = struct.unpack('%sf' % OBSERVATION_SIZE, observation_bin)

        obs = {'observation': obs, 'achieved_goal': np.array([obs[0]]),
               'desired_goal': np.array([1.0]),
               'non_noisy_obs': obs}
        is_success = 0
        if self.initial_obs is not None:
            if obs['achieved_goal'][0] > self.initial_obs['achieved_goal'][0]:
                is_success = 1

        done = False
        if self.initial_obs is not None:
            done = self._done(obs['achieved_goal'][0],
                              self.initial_obs['achieved_goal'][0])
        info = {'is_success': is_success}

        r = self.compute_reward(obs['achieved_goal'], obs['desired_goal'])
        self.step_ctr += 1
        return obs, r, done, info


if __name__ == "__main__":
    max_steps_per_episode = 2000  # 200 = 5s
    nao_env = NAOEnv()
    episode = 0
    most_recent_stability = 1.0

    while True:
        episode += 1
        nao_env.initial_obs = nao_env.reset()
        print("Episode", episode, "started")
        while nao_env.step_ctr < max_steps_per_episode:
            if (time.time() - nao_env.start_time) > 1:
                nao_env.fps = nao_env.fps_count
                nao_env.fps_count = 1
                nao_env.start_time = time.time()
            else:
                nao_env.fps_count += 1

            action = np.array([0.02 * (-0.5 + random.random())
                              for _ in range(ACTION_SIZE)])
            obs, r, done, info = nao_env.step(action)
            most_recent_stability = obs['achieved_goal'][0]
            print("Step:", nao_env.step_ctr, "Stability", most_recent_stability,
                  "Initial_Stability", nao_env.initial_obs['achieved_goal'][0])
            if done:
                print('Episode ended:', info, "FPS:", nao_env.fps)
                break
            if nao_env.step_ctr >= max_steps_per_episode:
                print("Max episode time reached: ", info, "FPS:", nao_env.fps)
            time.sleep(nao_env.delay)
