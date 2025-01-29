from pathlib import Path
from typing import Any, ClassVar, override

import numpy as np
from gymnasium.envs.mujoco.mujoco_env import MujocoEnv
from gymnasium.spaces import Box
from nao_interface import Nao
from numpy.typing import NDArray
from throwing import ThrowableObject

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

ACTUATOR_NAMES = [
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

DEFAULT_CAMERA_CONFIG = {
    "trackbodyid": 1,
    "distance": 4.0,
    "lookat": np.array((0.0, 0.0, 0.8925)),
    "elevation": -20.0,
}


class NaoBaseEnv(MujocoEnv):
    metadata: ClassVar[dict[str, Any]] = {
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
        throw_tomatoes: bool = False,
        fsr_scale: float = 0.019,
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
            str(Path.cwd().joinpath("model", "scene.xml")),
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
        self._actuation_mask = self._get_actuation_mask()
        self.action_space_size = len(ACTUATOR_NAMES)
        self.nao = Nao(self.model, self.data, fsr_scale=fsr_scale)

    def _get_actuation_mask(self) -> NDArray[np.bool_]:
        actuation_mask = np.zeros(self.model.nu, dtype=np.bool_)
        for name in ACTUATOR_NAMES:
            actuation_mask[self.model.actuator(name).id] = 1
        return actuation_mask

    @override
    def _set_action_space(self) -> Box:
        bounds = (
            np.stack(
                [self.model.actuator(name).ctrlrange for name in ACTUATOR_NAMES]
            )
            .copy()
            .astype(np.float32)
        )
        low, high = bounds.T
        self.action_space = Box(low=low, high=high, dtype=np.float32)
        return self.action_space

    def _get_obs(self) -> NDArray[np.floating]:
        force_sensing_resistors_right = self.nao.right_fsr_values().sum()
        force_sensing_resistors_left = self.nao.left_fsr_values().sum()

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
    def do_simulation(
        self,
        ctrl: NDArray[np.floating],
        n_frames: int,
    ) -> None:
        self.data.ctrl[self._actuation_mask] += ctrl
        self._step_mujoco_simulation(self.data.ctrl, n_frames)
