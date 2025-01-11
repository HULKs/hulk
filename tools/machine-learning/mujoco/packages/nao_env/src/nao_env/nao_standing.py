from pathlib import Path
from typing import ClassVar

import numpy as np
from gymnasium import utils
from gymnasium.envs.mujoco.mujoco_env import MujocoEnv
from gymnasium.spaces import Box
from nao_interface.nao_interface import Nao
from nao_interface.poses import PENALIZED_POSE
from throwing import ThrowableObject

DEFAULT_CAMERA_CONFIG = {
    "trackbodyid": 1,
    "distance": 4.0,
    "lookat": np.array((0.0, 0.0, 0.8925)),
    "elevation": -20.0,
}

OFFSET_QPOS = np.array(
    [
        0.0,
        0.0,
        0.0,
        0.0,
        0.09,
        -0.06,
        0.01,
        -0.002,
        0.0,
        0.09,
        -0.06,
        0.01,
        0.002,
        1.57,
        0.1,
        -1.57,
        0.0,
        0.0,
        1.57,
        -0.1,
        1.57,
        0.0,
        0.0,
    ]
)

HEAD_SET_HEIGHT = 5.11757778e-01


class NaoStanding(MujocoEnv, utils.EzPickle):
    metadata: ClassVar = {
        "render_modes": [
            "human",
            "rgb_array",
            "depth_array",
        ],
        "render_fps": 83,
    }

    def __init__(self, throw_tomatos: bool, **kwargs) -> None:
        observation_space = Box(
            low=-np.inf,
            high=np.inf,
            shape=(37,),
            dtype=np.float64,
        )

        MujocoEnv.__init__(
            self,
            str(Path.cwd().joinpath("model", "scene.xml")),
            frame_skip=4,
            observation_space=observation_space,
            default_camera_config=DEFAULT_CAMERA_CONFIG,
            **kwargs,
        )
        self.throw_tomatos = throw_tomatos
        self.projectile = ThrowableObject(
            self.model, self.data, "floor", "tomato"
        )
        self.current_step = 0
        utils.EzPickle.__init__(self, **kwargs)

    def _get_obs(self) -> np.ndarray:
        nao = Nao(self.model, self.data)
        return nao.data.sensordata

    def step(self, action):
        if self.projectile.has_ground_contact() and self.throw_tomatos:
            robot_site_id = self.model.site("Robot").id
            target = self.data.site_xpos[robot_site_id]
            alpha = self.current_step / 2500
            time_to_reach = 0.2 * (1 - alpha) + 0.1 * alpha
            self.projectile.random_throw(
                target, time_to_reach=time_to_reach, distance=1.0
            )

        last_action = self.data.ctrl.copy()
        self.do_simulation(action + OFFSET_QPOS, self.frame_skip)
        head_center_z = self.data.site("head_center").xpos[2]
        head_center_xy = self.data.site("head_center").xpos[:2]

        action_penalty = 0.1 * np.mean(np.square(self.data.ctrl - last_action))

        head_ctrl = 2.0 * np.square(head_center_z - HEAD_SET_HEIGHT) + np.mean(
            np.square(head_center_xy)
        )

        if self.render_mode == "human":
            self.render()

        terminated = head_center_z < 0.3
        reward = 0.05 - head_ctrl - action_penalty

        self.current_step += 1
        return (
            self._get_obs(),
            reward,
            terminated,
            False,
            {},
        )

    def reset_model(self):
        self.current_step = 0
        self.set_state(
            self.init_qpos,
            self.init_qvel,
        )
        nao = Nao(self.model, self.data)
        nao.reset(PENALIZED_POSE)
        return self._get_obs()
