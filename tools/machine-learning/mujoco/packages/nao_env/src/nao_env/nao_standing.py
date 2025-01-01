from pathlib import Path

from nao_interface.nao_interface import Nao
from nao_interface.poses import PENALIZED_POSE
import numpy as np
from gymnasium import utils
from gymnasium.envs.mujoco.mujoco_env import MujocoEnv
from gymnasium.spaces import Box
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
    metadata = {
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
            self.model, self.data, "floor", "projectile"
        )
        utils.EzPickle.__init__(self, **kwargs)

    def _get_obs(self) -> np.ndarray:
        nao = Nao(self.model, self.data)
        return nao.data.sensordata

    def step(self, action):
        head_center_id = self.model.site("head_center").id
        if self.projectile.throw_has_ended() and self.throw_tomatos:
            target = self.data.site_xpos[head_center_id]
            self.projectile.random_throw(target, 0.2, 1.0)

        previous_ctrl = self.data.ctrl.copy()
        self.do_simulation(action + OFFSET_QPOS, self.frame_skip)
        head_center_z = self.data.site("head_center").xpos[2]
        head_center_xy = self.data.site("head_center").xpos[:2]
        joint_speed = (self.data.ctrl - previous_ctrl)
        diff_ctrl = 0.001 * np.mean(np.square(joint_speed))
        head_ctrl = 2.0 * np.square(
            head_center_z - HEAD_SET_HEIGHT) + np.mean(np.square(head_center_xy))

        if self.render_mode == "human":
            self.render()

        terminated = head_center_z < 0.3
        reward = - diff_ctrl - head_ctrl
        if terminated:
            reward -= 10.0
        # reward = self.model.opt.timestep

        return (
            self._get_obs(),
            reward,
            terminated,
            False,
            {
                "diff_ctrl": diff_ctrl,
                "head_ctrl": head_ctrl,
            },
        )

    def reset_model(self):
        self.set_state(
            self.init_qpos,
            self.init_qvel,
        )
        nao = Nao(self.model, self.data)
        nao.reset(PENALIZED_POSE)
        return self._get_obs()
