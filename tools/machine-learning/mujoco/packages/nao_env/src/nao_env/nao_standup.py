from pathlib import Path
from typing import ClassVar

import numpy as np
from gymnasium import utils
from gymnasium.envs.mujoco.mujoco_env import MujocoEnv
from gymnasium.spaces import Box

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

        return np.concatenate([
            data.sensordata.flat[:-8],
            force_sensing_resistors_right.flat,
            force_sensing_resistors_left.flat,
        ])

    def step(self, action):
        self.do_simulation(action, self.frame_skip)
        data = self.data

        head_center_id = self.model.site("head_center").id
        head_center_z = data.site_xpos[head_center_id][2]
        uph_cost = (head_center_z - 0) / self.model.opt.timestep

        quad_ctrl_cost = 0.1 * np.square(data.ctrl).sum()
        quad_impact_cost = 0.5e-6 * np.square(data.cfrc_ext).sum()
        quad_impact_cost = min(quad_impact_cost, 10)
        reward = uph_cost - quad_ctrl_cost - quad_impact_cost + 1

        if self.render_mode == "human":
            self.render()

        return (
            self._get_obs(),
            reward,
            False,
            False,
            {
                "reward_linup": uph_cost,
                "reward_quadctrl": -quad_ctrl_cost,
                "reward_impact": -quad_impact_cost,
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
