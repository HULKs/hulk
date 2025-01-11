from pathlib import Path
from typing import ClassVar

import numpy as np
import rewards
from gymnasium import utils
from gymnasium.envs.mujoco.mujoco_env import MujocoEnv
from gymnasium.spaces import Box
from nao_interface import Nao

DEFAULT_CAMERA_CONFIG = {
    "trackbodyid": 1,
    "distance": 4.0,
    "lookat": np.array((0.0, 0.0, 0.8925)),
    "elevation": -20.0,
}


class NaoStandup(MujocoEnv, utils.EzPickle):
    metadata: ClassVar = {
        "render_modes": [
            "human",
            "rgb_array",
            "depth_array",
        ],
        "render_fps": 67,
    }

    def __init__(self, **kwargs) -> None:
        observation_space = Box(
            low=-np.inf,
            high=np.inf,
            shape=(31,),
            dtype=np.float64,
        )

        MujocoEnv.__init__(
            self,
            str(Path.cwd().joinpath("model", "scene.xml")),
            5,
            observation_space=observation_space,
            default_camera_config=DEFAULT_CAMERA_CONFIG,
            **kwargs,
        )
        utils.EzPickle.__init__(self, **kwargs)

    def _get_obs(self) -> np.ndarray:
        data = self.data

        force_sensing_resistors_right = np.sum(data.sensordata[-8:-4])
        force_sensing_resistors_left = np.sum(data.sensordata[-4:])

        return np.concatenate(
            [
                data.sensordata.flat[:-8],
                force_sensing_resistors_right.flat,
                force_sensing_resistors_left.flat,
            ]
        )

    def step(self, action):
        self.do_simulation(action, self.frame_skip)
        nao = Nao(self.model, self.data)

        head_elevation_reward = rewards.head_height(nao)
        control_amplitude_penalty = 0.1 * rewards.ctrl_amplitude(nao)
        impact_penalty = min(0.5e-6 * rewards.impact_forces(nao), 10)

        reward = (
            head_elevation_reward
            - control_amplitude_penalty
            - impact_penalty
            + 1
        )

        if self.render_mode == "human":
            self.render()

        return (
            self._get_obs(),
            reward,
            False,
            False,
            {
                "head_elevation_reward": head_elevation_reward,
                "control_amplitude_penalty": control_amplitude_penalty,
                "impact_penalty": impact_penalty,
            },
        )

    def reset_model(self):
        half_random_offset = 0.03
        face_down_keyframe_qpos = [
            0.452845,
            0.219837,
            0.0556939,
            0.710551,
            -0.0810676,
            0.693965,
            0.0834173,
            -0.000571484,
            0.0239414,
            0.000401842,
            -3.89047e-05,
            -0.00175077,
            0.357233,
            0.0114063,
            0.000212495,
            0.000422366,
            3.92127e-05,
            -0.00133669,
            0.356939,
            0.0112884,
            -0.000206283,
            1.46985,
            0.110264,
            0.000766453,
            -0.034298,
            3.65047e-05,
            1.47067,
            -0.110094,
            -0.00201064,
            0.0342998,
            -0.00126886,
        ]
        self.set_state(
            face_down_keyframe_qpos
            + self.np_random.uniform(
                low=-half_random_offset,
                high=half_random_offset,
                size=self.model.nq,
            ),
            self.init_qvel
            + self.np_random.uniform(
                low=-half_random_offset,
                high=half_random_offset,
                size=self.model.nv,
            ),
        )
        return self._get_obs()
