from pathlib import Path
from typing import ClassVar

import numpy as np
import walking_engine
from gymnasium import utils
from gymnasium.envs.mujoco.mujoco_env import MujocoEnv
from gymnasium.spaces import Box
from nao_interface.nao_interface import Nao
from nao_interface.poses import PENALIZED_POSE
from throwing import ThrowableObject
from transforms.transforms import Pose2
from walking_engine import (
    Control,
    Measurements,
    Parameters,
)
from walking_engine.walking_types import Feet, Side, State

DEFAULT_CAMERA_CONFIG = {
    "trackbodyid": 1,
    "distance": 4.0,
    "lookat": np.array((0.0, 0.0, 0.8925)),
    "elevation": -20.0,
}

HEAD_SET_HEIGHT = 5.11757778e-01

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


def initial_state() -> State:
    return State(
        t=1.0,
        support_side=Side.RIGHT,
        start_feet=Feet(
            support_sole=Pose2(),
            swing_sole=Pose2(),
        ),
        end_feet=Feet(
            support_sole=Pose2(),
            swing_sole=Pose2(),
        ),
    )


class NaoWalking(MujocoEnv, utils.EzPickle):
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

        self.control = Control(forward=0.06, left=0.0, turn=0.0)
        self.state = initial_state()
        self.parameter = Parameters(
            sole_pressure_threshold=0.5,
            min_step_duration=0.25,
            step_duration=0.25,
            foot_lift_apex=0.015,
            foot_offset_left=0.052,
            foot_offset_right=-0.052,
            base_height=0.23,
        )

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
            self.projectile.random_throw(target, time_to_reach, 1.0)

        last_action = self.data.ctrl.copy()
        self.do_simulation(action, self.frame_skip)

        if self.render_mode == "human":
            self.render()

        head_center_z = self.data.site("head_center").xpos[2]
        action_penalty = 0.1 * np.mean(np.square(self.data.ctrl - last_action))
        head_ctrl = 2.0 * np.square(head_center_z - HEAD_SET_HEIGHT)

        reward = 0.05 - head_ctrl - action_penalty
        terminated = head_center_z < 0.3

        self.current_step += 1
        return (
            self._get_obs(),
            reward,
            terminated,
            False,
            {},
        )

    def do_simulation(self, ctrl, n_frames):
        nao = Nao(self.model, self.data)

        fsr_positions = [
            "rear_left",
            "rear_right",
            "front_left",
            "front_right",
        ]

        right_pressure = (
            sum(
                nao.data.sensor(f"force_sensitive_resistors.right.{pos}").data
                for pos in fsr_positions
            )
            / 10.0
        )
        left_pressure = (
            sum(
                nao.data.sensor(f"force_sensitive_resistors.left.{pos}").data
                for pos in fsr_positions
            )
            / 10.0
        )

        measurements = Measurements(left_pressure, right_pressure)
        nao.data.ctrl[:] = OFFSET_QPOS

        if (
            measurements.pressure_left > 0.0
            or measurements.pressure_right > 0.0
        ):
            dt = self.model.opt.timestep * n_frames
            apply_walking(
                nao, self.parameter, self.state, measurements, self.control, dt
            )
        nao.data.ctrl[:] += ctrl

        self._step_mujoco_simulation(nao.data.ctrl, n_frames)

    def reset_model(self):
        self.current_step = 0
        self.state = initial_state()
        self.set_state(
            self.init_qpos,
            self.init_qvel,
        )
        nao = Nao(self.model, self.data)
        nao.reset(PENALIZED_POSE)
        return self._get_obs()


def apply_walking(
    nao: Nao,
    parameters: Parameters,
    state: State,
    measurements: Measurements,
    control: Control,
    dt: float,
):
    state, left_sole, left_lift, right_sole, right_lift = walking_engine.step(
        state,
        measurements,
        control,
        dt,
        parameters,
    )

    left_leg_joints, right_leg_joints = walking_engine.joint_command(
        left_sole, right_sole, left_lift, right_lift
    )

    nao.actuators.left_leg.ankle_pitch += left_leg_joints.ankle_pitch
    nao.actuators.left_leg.ankle_roll += left_leg_joints.ankle_roll
    nao.actuators.left_leg.knee_pitch += left_leg_joints.knee_pitch
    nao.actuators.left_leg.hip_pitch += left_leg_joints.hip_pitch
    nao.actuators.left_leg.hip_roll += left_leg_joints.hip_roll
    nao.actuators.left_leg.hip_yaw_pitch += left_leg_joints.hip_yaw_pitch

    nao.actuators.right_leg.ankle_pitch += right_leg_joints.ankle_pitch
    nao.actuators.right_leg.ankle_roll += right_leg_joints.ankle_roll
    nao.actuators.right_leg.knee_pitch += right_leg_joints.knee_pitch
    nao.actuators.right_leg.hip_pitch += right_leg_joints.hip_pitch
    nao.actuators.right_leg.hip_roll += right_leg_joints.hip_roll
