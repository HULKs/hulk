from pathlib import Path
from typing import Any, ClassVar, override

import numpy as np
import rewards
from gymnasium import utils
from gymnasium.envs.mujoco.mujoco_env import MujocoEnv
from gymnasium.spaces import Box
from nao_interface import Nao
from numpy.typing import NDArray

DEFAULT_CAMERA_CONFIG = {
    "trackbodyid": 1,
    "distance": 4.0,
    "lookat": np.array((0.0, 0.0, 0.8925)),
    "elevation": -20.0,
}

SENSOR_NAMES = [
    "accelerometer",
    "gyroscope",
    "head.yaw"
    "head.pitch"
    "left_leg.hip_yaw_pitch"
    "left_leg.hip_roll"
    "left_leg.hip_pitch"
    "left_leg.knee_pitch"
    "left_leg.ankle_pitch"
    "left_leg.ankle_roll"
    "right_leg.hip_yaw_pitch"
    "right_leg.hip_roll"
    "right_leg.hip_pitch"
    "right_leg.knee_pitch"
    "right_leg.ankle_pitch"
    "right_leg.ankle_roll"
    "left_arm.shoulder_pitch"
    "left_arm.shoulder_roll"
    "left_arm.elbow_yaw"
    "left_arm.elbow_roll"
    "left_arm.wrist_yaw"
    "right_arm.shoulder_pitch"
    "right_arm.shoulder_roll"
    "right_arm.elbow_yaw"
    "right_arm.elbow_roll"
    "right_arm.wrist_yaw",
]


class NaoStandup(MujocoEnv, utils.EzPickle):
    metadata: ClassVar = {
        "render_modes": [
            "human",
            "rgb_array",
            "depth_array",
        ],
        "render_fps": 67,
    }

    def __init__(self, **kwargs: Any) -> None:
        observation_space = Box(
            low=-np.inf,
            high=np.inf,
            shape=(31,),
            dtype=np.float64,
        )

        MujocoEnv.__init__(
            self,
            str(Path.cwd() / "model/scene.xml"),
            5,
            observation_space=observation_space,
            default_camera_config=DEFAULT_CAMERA_CONFIG,
            **kwargs,
        )
        utils.EzPickle.__init__(self, **kwargs)

    @override
    def _get_obs(self) -> NDArray[np.floating]:
        nao = Nao(self.model, self.data)

        force_sensing_resistors_right = (
            nao.force_sensing_resistors_right().sum()
        )
        force_sensing_resistors_left = nao.force_sensing_resistors_left().sum()

        return np.concatenate(
            [self.data.sensor(sensor_name).data for sensor_name in SENSOR_NAMES]
            + [
                force_sensing_resistors_right,
                force_sensing_resistors_left,
            ],
        )

    @override
    def step(self, action: NDArray[np.floating]) -> tuple:
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

    @override
    def reset_model(self) -> NDArray[np.floating]:
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
