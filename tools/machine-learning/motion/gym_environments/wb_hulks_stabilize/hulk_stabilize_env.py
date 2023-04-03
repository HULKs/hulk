import gym
from gym import spaces
from gym.utils import seeding
import numpy as np
import random
import struct
import time
from websocket import create_connection

# Code-snippet for Scilab-rl custom_envs/register_envs.py
#
# register(
#     id='wb-hulks-stabilize-v0',
#     entry_point='custom_envs.wb_hulks_stabilize.hulk_stabilize_env:NAOEnvMaker',
#     kwargs=kwargs,
#     max_episode_steps=10000,
# )

CACHED_ENV = None
ACTION_SIZE = 2
FULL_ACTION_SIZE = 2 * 26
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
    def __init__(self, render_mode='none', ik=1, reward_type='sparse'):
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
        self.action_space = spaces.Box(-3., 3.,
                                       shape=(ACTION_SIZE,), dtype='float32')
        self.observation_space = spaces.Dict(dict(
            desired_goal=spaces.Box(-3., 3., shape=(1,), dtype='float32'),
            achieved_goal=spaces.Box(-3., 3., shape=(1,), dtype='float32'),
            observation=spaces.Box(-3., 3., shape=(OBSERVATION_SIZE,), dtype='float32'),))
        self.initial_obs = None

        #start webots

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
        time.sleep(1.00)
        obs, r, done, info = self.step(action)
        return obs

    def compute_reward(self, achieved_goal, goal):
        if achieved_goal[0] == 1.0:
            return np.array(1.0)
        else:
            return np.array(-1.0)

    def _done(self, most_recent_stability, initial_stability):
        if most_recent_stability is None or initial_stability is None:
            return False
        else:
            if (most_recent_stability == 1.0 and initial_stability == 0.0):
                #print("Robot stood up")
                return True
            if (most_recent_stability == 0.0 and initial_stability != 0.0):
                #print("Robot fell")
                return True
            return False

    def step(self, action):
        # send action
        #print("Action", action)
        #action = [0.02 * (-0.5 + random.random()) for _ in range(ACTION_SIZE)]
        full_action = [0.0 for _ in range(FULL_ACTION_SIZE)]
        full_action[12] = action[0]
        full_action[24] = action[1]
        action_bin = struct.pack('%sf' % len(full_action), *full_action)
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
