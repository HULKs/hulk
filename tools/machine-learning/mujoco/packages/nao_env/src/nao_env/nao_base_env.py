from collections.abc import Sequence
from pathlib import Path
from typing import Any, ClassVar, override

import mujoco
import numpy as np
from gymnasium.envs.mujoco.mujoco_env import MujocoEnv
from gymnasium.spaces import Box
from nao_interface import Nao
from numpy.typing import NDArray
from throwing import ThrowableObject

DEFAULT_ACTUATOR_NAMES = [
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
    "hip_yaw_pitch",
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
        sensor_delay: int = 0,
        actuator_names: Sequence[str] = DEFAULT_ACTUATOR_NAMES,
        **kwargs: Any,
    ) -> None:
        observation_space = Box(
            low=-np.inf,
            high=np.inf,
            shape=(31,),
            dtype=np.float64,
        )
        self.actuator_names = actuator_names
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
        self.nao = Nao(
            self.model,
            self.data,
            fsr_scale=fsr_scale,
            position_sensor_delay=sensor_delay,
            fsr_sensor_delay=sensor_delay,
            gyroscope_sensor_delay=sensor_delay,
            accelerometer_sensor_delay=sensor_delay,
            gyroscope_noise=np.array([0.003889, 0.005522, 0.002381]),
            accelerometer_noise=np.array([0.063137, 0.029709, 0.087581]),
        )

    def initialize_terrain(
        self,
        *,
        max_height: float = 0.1,
        step_height: float = 0.01,
        hfield_id: int = 0,
    ) -> None:
        num_rows = self.model.hfield_nrow[hfield_id]
        num_cols = self.model.hfield_ncol[hfield_id]

        x = np.linspace(-5, 5, num_cols)
        y = np.linspace(-5, 5, num_rows)

        X, Y = np.meshgrid(x, y)
        Z = 1 - (np.cos(X) * np.cos(Y) + 1) / 2
        num_steps = int(max_height / step_height)
        discrete_Z = np.round(Z * num_steps) / num_steps * max_height

        flattened_terrain = discrete_Z.flatten()

        self.model.hfield_data = flattened_terrain

    @override
    def _set_action_space(self) -> Box:
        bounds = (
            np.stack(
                [
                    self.model.actuator(name).ctrlrange
                    for name in self.actuator_names
                ]
            )
            .copy()
            .astype(np.float32)
        )
        low, high = bounds.T
        self.action_space = Box(low=low, high=high, dtype=np.float32)
        return self.action_space

    def _get_obs(self) -> NDArray[np.floating]:
        force_sensing_resistors_left = self.nao.left_fsr().sum(keepdims=True)
        force_sensing_resistors_right = self.nao.right_fsr().sum(keepdims=True)

        positions = self.nao.position_encoders()
        gyroscopes = self.nao.gyroscope()
        accelerometer = self.nao.accelerometer()

        return np.concatenate(
            [
                positions,
                gyroscopes,
                accelerometer,
                force_sensing_resistors_left,
                force_sensing_resistors_right,
            ]
        )

    @override
    def _step_mujoco_simulation(self, ctrl: NDArray, n_frames: int) -> None:
        self.nao.data.ctrl = ctrl

        mujoco.mj_step(self.model, self.data, nstep=n_frames)

        # As of MuJoCo 2.0, force-related quantities like cacc are not computed
        # unless there's a force sensor in the model.
        # See https://github.com/openai/gym/issues/1541
        mujoco.mj_rnePostConstraint(self.model, self.data)

    @override
    def do_simulation(
        self,
        ctrl: NDArray[np.floating],
        n_frames: int,
    ) -> None:
        if ctrl.shape != (len(self.actuator_names),):
            raise ActionShapeError(ctrl.shape, len(self.actuator_names))
        self.nao.actuator_control.set_from_numpy(ctrl, self.actuator_names)
        self._step_mujoco_simulation(self.data.ctrl, n_frames)
        self.nao.update_sensors()


class ActionShapeError(Exception):
    def __init__(
        self,
        shape: tuple,
        action_space_size: int,
    ) -> None:
        super().__init__(
            f"Action shape {shape} does not"
            f"match action space size {action_space_size}"
        )
