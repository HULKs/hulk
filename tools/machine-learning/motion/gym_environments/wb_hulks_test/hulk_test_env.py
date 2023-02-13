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
#     id='wb-hulks-test-v0',
#     entry_point='custom_envs.wb_hulks_test.hulk_test_env:NAOEnvMaker',
#     kwargs=kwargs,
#     max_episode_steps=100,
# )

CACHED_ENV = None
ACTION_SIZE = 1
FULL_ACTION_SIZE = 2 * 26
OBSERVATION_SIZE = 1
FULL_OBSERVATION_SIZE = (3 * 26) + (2 * 8) + 2


class NAOEnvMaker:
    def __new__(cls, *args, **kwargs):
        global CACHED_ENV
        if CACHED_ENV is None:
            return NAOEnv(*args, **kwargs)
        else:
            print('\033[92m' + 'Using cached Env' + '\033[0m')
            return CACHED_ENV


class NAOEnv(gym.GoalEnv):
    def __init__(self, render_mode='none', ik=0, reward_type='sparse'):
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

    def compute_reward(self, achieved_goal, goal, info):
        return 1.0 - abs(achieved_goal[0] - goal[0])
#    	if abs(achieved_goal[0] - goal[0]) < 0.1:
#    	    return 1.0
#    	else:
#    	    return 0.0
    	

    def _done(self, most_recent_stability, initial_stability):
        return False

    def step(self, action):
        # send action
        #print("Action", action)
        #action = [0.02 * (-0.5 + random.random()) for _ in range(ACTION_SIZE)]
        full_action = [0.0 for _ in range(FULL_ACTION_SIZE)]
        full_action[2] = action[0]
        action_bin = struct.pack('%sf' % len(full_action), *full_action)
        self.ws.send_binary(action_bin)

        # receive observation
        observation_bin = self.ws.recv()
        obs = struct.unpack('%sf' % FULL_OBSERVATION_SIZE, observation_bin)

        obs = {'observation': np.array([obs[4]]), 'achieved_goal': np.array([obs[4]]),
               'desired_goal': np.array([0.0]),
               'non_noisy_obs': obs}
        is_success = 0
        if self.initial_obs is not None:
            if abs(obs['achieved_goal'][0] - obs['desired_goal'][0]) < 0.2:
                is_success = 1

        done = False
        info = {'is_success': is_success}

        r = self.compute_reward(obs['achieved_goal'], obs['desired_goal'], info)
        self.step_ctr += 1
        #print(obs['achieved_goal'], obs['desired_goal'], action, r)
        return obs, r, done, info

