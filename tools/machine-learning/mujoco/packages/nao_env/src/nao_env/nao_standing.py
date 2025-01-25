from pathlib import Path
from typing import Any, ClassVar, override

import numpy as np
import rewards
from gymnasium import utils
from gymnasium.envs.mujoco.mujoco_env import MujocoEnv
from gymnasium.spaces import Box
from nao_interface.nao_interface import Nao
from nao_interface.poses import PENALIZED_POSE
from numpy.typing import NDArray
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
    ],
)

HEAD_SET_HEIGHT = 0.51

SENSOR_NAMES = [
    "accelerometer",
    "gyroscope",
    "head.yaw",
    "head.pitch",
    "left_leg.hip_yaw_pitch",
    "left_leg.hip_roll",
    "left_leg.hip_pitch",
    "left_leg.knee_pitch",
    "left_leg.ankle_pitch",
    "left_leg.ankle_roll",
    "right_leg.hip_roll",
    "right_leg.hip_pitch",
    "right_leg.knee_pitch",
    "right_leg.ankle_pitch",
    "right_leg.ankle_roll",
    "left_arm.shoulder_pitch",
    "left_arm.shoulder_roll",
    "left_arm.elbow_yaw",
    "left_arm.elbow_roll",
    "left_arm.wrist_yaw",
    "right_arm.shoulder_pitch",
    "right_arm.shoulder_roll",
    "right_arm.elbow_yaw",
    "right_arm.elbow_roll",
    "right_arm.wrist_yaw",
]


class NaoStanding(MujocoEnv, utils.EzPickle):
    metadata: ClassVar = {
        "render_modes": [
            "human",
            "rgb_array",
            "depth_array",
        ],
        "render_fps": 83,
    }

    def __init__(
        self,
        *,
        throw_tomatoes: bool,
        **kwargs: Any,
    ) -> None:
        observation_space = Box(
            low=-np.inf,
            high=np.inf,
            shape=(31,),
            dtype=np.float64,
        )

        MujocoEnv.__init__(
            self,
            str(Path.cwd() / "model/scene.xml"),
            frame_skip=4,
            observation_space=observation_space,
            default_camera_config=DEFAULT_CAMERA_CONFIG,
            **kwargs,
        )
        self.throw_tomatoes = throw_tomatoes
        self.projectile = ThrowableObject(
            model=self.model,
            data=self.data,
            plane_body="floor",
            throwable_body="tomato",
        )
        self.current_step = 0
        self.termination_penalty = 10.0
        utils.EzPickle.__init__(self, **kwargs)

    @override
    def _get_obs(self) -> NDArray[np.floating]:
        nao = Nao(self.model, self.data)

        force_sensing_resistors_right = nao.right_fsr_values().sum()
        force_sensing_resistors_left = nao.left_fsr_values().sum()

        sensors = np.concatenate(
            [
                self.data.sensor(sensor_name).data
                for sensor_name in SENSOR_NAMES
            ],
        )
        fsrs = np.array(
            [force_sensing_resistors_right, force_sensing_resistors_left],
        )

        return np.concatenate([sensors, fsrs])

    @override
    def step(self, action: NDArray[np.floating]) -> tuple:
        self.current_step += 1
        nao = Nao(self.model, self.data)

        if self.throw_tomatoes and self.projectile.has_ground_contact():
            robot_site_id = self.model.site("Robot").id
            target = self.data.site_xpos[robot_site_id]
            alpha = self.current_step / 2500
            time_to_reach = 0.2 * (1 - alpha) + 0.1 * alpha
            self.projectile.random_throw(
                target,
                time_to_reach=time_to_reach,
                distance=1.0,
            )

        last_actuator_force = self.data.actuator_force.copy()
        self.do_simulation(action + OFFSET_QPOS, self.frame_skip)
        head_center_z = self.data.site("head_center").xpos[2]

        torque_penalty = 0.01 * rewards.torque_change_rate(
            nao,
            last_actuator_force,
        )
        head_over_torso_penalty = 1.0 * rewards.head_over_torso_error(nao)

        if self.render_mode == "human":
            self.render()

        terminated = head_center_z < 0.3
        reward = 0.05 - torque_penalty - head_over_torso_penalty

        if terminated:
            reward -= self.termination_penalty

        self.current_step += 1
        return (
            self._get_obs(),
            reward,
            terminated,
            False,
            {},
        )

    @override
    def reset_model(self) -> NDArray[np.floating]:
        self.current_step = 0
        self.set_state(
            self.init_qpos,
            self.init_qvel,
        )
        nao = Nao(self.model, self.data)
        nao.reset(PENALIZED_POSE)
        return self._get_obs()
