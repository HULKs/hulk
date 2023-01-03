import gym
from gym import spaces
from gym.utils import seeding
import numpy as np
import random
import struct
import time
from websocket import create_connection


CACHED_ENV = None
ACTION_SIZE = 2* 26
OBSERVATION_SIZE = (3 * 26) + (2 * 8) + 2

class NAOEnvMaker:
    def __new__(cls, *args, **kwargs):
        global CACHED_ENV
        if CACHED_ENV is None:
            return NAOEnv(*args, **kwargs)
        else:
            print('\033[92m' + 'Using cached Env' + '\033[0m')
            return CACHED_ENV

class NAOEnv(): #gym.GoalEnv):
    def __init__(self, render_mode='none', reward_type='sparse'):
        print('\033[92m' + 'Creating new Env' + '\033[0m')
        #render = render_mode == 'human'
        self.reward_type = reward_type
        self.seed()
        self.delay = 0.01
        self.fps = 0
        self.fps_count = 0
        self.goal = [1.0]
        self.step_ctr = 0
        self.initial_stability = 0.0
        self.start_time = time.time()
        self.ws = create_connection("ws://localhost:9990")
        global CACHED_ENV
        CACHED_ENV = self
        self.reset()

    def seed(self, seed=None):
        self.np_random, seed = seeding.np_random(seed)
        return [seed]

    def render(self, mode='none', **kwargs):
        return

    def _get_obs(self):
        achieved_goal = [0.0]
        obs = np.concatenate([achieved_goal,
                                  [0.0,0.0,0.0], # imu, fsr, current_angles, target_angles, stiffnesses ?
                                  ])

        obs = {'observation': obs.copy(), 'achieved_goal': achieved_goal.copy(),
               'desired_goal': np.array(self.goal.copy()),
               'non_noisy_obs': obs.copy()}
        return obs

    def _sample_goal(self):
        self.target = 4.0
        return self.target

    def reset(self):
        # reset stuff

        action = np.array([0.0 for _ in range(ACTION_SIZE)])
        (obs, r, done, info) = self.step(action)
        self.step_ctr = 0
        self.initial_stability = obs[0]
        desired_stability = 1.0

        achieved_goal = [self.initial_stability]
        self.goal = [desired_stability]

        obs = {'observation': obs, 'achieved_goal': achieved_goal,
               'desired_goal': np.array(self.goal),
               'non_noisy_obs': obs}
        return obs

    def compute_reward(self, achieved_goal, goal, info):
        return np.array(-1)

    def _is_success(self, achieved_goal, desired_goal):
        return 0

    def step(self, action):
        # send action
        #action = [0.02 * (-0.5 + random.random()) for _ in range(ACTION_SIZE)]
        action_bin = struct.pack('%sf' % len(action), *action)
        self.ws.send_binary(action_bin)

        #receive observation
        observation_bin = self.ws.recv()
        obs = struct.unpack('%sf' % OBSERVATION_SIZE, observation_bin)

        #obs = {'observation': obs.copy(), 'achieved_goal': achieved_goal.copy(),
        #       'desired_goal': np.array(self.goal.copy()),
        #       'non_noisy_obs': obs.copy()}

        is_success = self.goal  #self._is_success(obs['achieved_goal'], obs['desired_goal'])
        done = bool(is_success)
        info = {'is_success': is_success}

        r = 0.0 #self.compute_reward(obs['achieved_goal'], obs['desired_goal'], {})
        self.step_ctr += 1
        return obs, r, done, info


if __name__ == "__main__":
    nao_env = NAOEnv()
    while True:
        if (time.time()-nao_env.start_time) > 1 :
            nao_env.fps = nao_env.fps_count
            nao_env.fps_count = 1
            nao_env.start_time = time.time()
            print("FPS:", nao_env.fps)
        else:
            nao_env.fps_count += 1

        action = np.array([0.02 * (-0.5 + random.random()) for _ in range(ACTION_SIZE)])
        (obs, r, done, info) = nao_env.step(action)
        print("Reward:", r, "Done:", done, "Info:", info, "Step:", nao_env.step_ctr, "Stability:", obs[0],"Step_t",obs[1])
        time.sleep(nao_env.delay)
