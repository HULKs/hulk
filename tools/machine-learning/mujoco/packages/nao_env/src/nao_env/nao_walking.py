from pathlib import Path
from typing import Any, ClassVar, override

import numpy as np
import walking_engine
from gymnasium import utils
from gymnasium.envs.mujoco.mujoco_env import MujocoEnv
from gymnasium.spaces import Box
from nao_interface.nao_interface import Nao
from nao_interface.poses import READY_POSE
from numpy.typing import NDArray
from rewards import (
    ConstantReward,
    ConstantSupportFootOrientationPenalty,
    ConstantSupportFootPositionPenalty,
    ControlAmplitudePenalty,
    HeadOverTorsoPenalty,
    JerkPenalty,
    RewardComposer,
    RewardContext,
    SwingFootDestinationReward,
    TorqueChangeRatePenalty,
    XDistanceReward,
)
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
    # "type": 1,
    "elevation": -20.0,
}

HEAD_THRESHOLD_HEIGHT = 0.4

OFFSET_QPOS = np.array(
    [
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
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


def initial_state(parameters: Parameters) -> State:
    return State(
        t=1.0,
        support_side=Side.RIGHT,
        start_feet=Feet(
            support_sole=Pose2(0.0, parameters.foot_offset_right, 0.0),
            swing_sole=Pose2(0.0, parameters.foot_offset_left, 0.0),
        ),
        end_feet=Feet(
            support_sole=Pose2(0.0, parameters.foot_offset_right, 0.0),
            swing_sole=Pose2(0.0, parameters.foot_offset_left, 0.0),
        ),
    )


class NaoWalking(MujocoEnv, utils.EzPickle):
    metadata: ClassVar[dict[str, Any]] = {
        "render_modes": [
            "human",
            "rgb_array",
            "depth_array",
        ],
        "render_fps": 83,
    }

    def __init__(self, *, throw_tomatoes: bool, **kwargs: Any) -> None:
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
        self.current_step = 0

        self.parameters = Parameters(
            sole_pressure_threshold=0.5,
            min_step_duration=0.25,
            step_duration=0.25,
            foot_lift_apex=0.015,
            foot_offset_left=0.052,
            foot_offset_right=-0.052,
            walk_height=0.23,
            torso_tilt=0.055,
            arm_pitch_factor=1.0,
        )
        self.state = initial_state(self.parameters)

        self.enable_walking = True
        self.termination_penalty = 30.0
        self.initialization_rounds = 2

        self.reward = (
            RewardComposer()
            .add(0.03, ConstantReward())
            .add(-0.0001, JerkPenalty(self.dt))
            .add(0.08, SwingFootDestinationReward(self.dt))
            .add(-0.0001, TorqueChangeRatePenalty(self.model.nu, self.dt))
            .add(None, HeadOverTorsoPenalty())  # -10.0
            .add(None, XDistanceReward())  # 1.0
            .add(-0.5, ConstantSupportFootPositionPenalty())
            .add(-0.5, ConstantSupportFootOrientationPenalty())
            .add(-0.001, ControlAmplitudePenalty())
        )

        utils.EzPickle.__init__(self, **kwargs)

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
        nao = Nao(self.model, self.data)
        robot_position = self.data.site("Robot").xpos

        if self.projectile.has_ground_contact() and self.throw_tomatoes:
            alpha = self.current_step / 2500
            time_to_reach = 0.2 * (1 - alpha) + 0.1 * alpha
            self.projectile.random_throw(
                robot_position,
                time_to_reach=time_to_reach,
                distance=1.0,
            )

        self.do_simulation(action, self.frame_skip)

        if self.mujoco_renderer.viewer is not None:
            self.mujoco_renderer.viewer.cam.lookat[:] = robot_position
        if self.render_mode == "human":
            self.render()

        distinct_rewards = self.reward.rewards(
            RewardContext(nao, self.state, action)
        )
        reward = sum(distinct_rewards.values())

        head_center_z = self.data.site("head_center").xpos[2]
        terminated = head_center_z < HEAD_THRESHOLD_HEIGHT

        if terminated:
            reward -= self.termination_penalty

        self.current_step += 1
        return (self._get_obs(), reward, terminated, False, distinct_rewards)

    @override
    def do_simulation(
        self,
        ctrl: NDArray[np.floating],
        n_frames: int,
    ) -> None:
        nao = Nao(self.model, self.data)

        right_pressure = nao.right_fsr_values().sum()
        left_pressure = nao.left_fsr_values().sum()

        measurements = Measurements(left_pressure, right_pressure)
        nao.data.ctrl[:] = OFFSET_QPOS

        if self.enable_walking and (
            measurements.pressure_left > 0.0
            or measurements.pressure_right > 0.0
        ):
            dt = self.model.opt.timestep * n_frames
            control = Control(
                forward=0.06 * min(1, self.data.time / 4.0),
                turn=0.0,
                left=0.0,
            )
            apply_walking(
                nao,
                parameters=self.parameters,
                state=self.state,
                measurements=measurements,
                control=control,
                dt=dt,
            )
        nao.data.ctrl[:] += ctrl

        self._step_mujoco_simulation(nao.data.ctrl, n_frames)

    @override
    def reset_model(self) -> NDArray[np.floating]:
        self.current_step = 0
        self.state = initial_state(self.parameters)
        self.reward.reset()
        self.set_state(
            self.init_qpos,
            self.init_qvel,
        )
        nao = Nao(self.model, self.data)
        nao.reset(READY_POSE)

        measurements = Measurements(
            nao.left_fsr_values().sum(),
            nao.right_fsr_values().sum(),
        )

        apply_walking(
            nao,
            self.parameters,
            self.state,
            measurements,
            Control(0.0, 0.0, 0.0),
            0.0,
        )

        self.enable_walking = False
        self.do_simulation(
            np.zeros(self.model.nu),
            self.frame_skip * self.initialization_rounds,
        )
        self.enable_walking = True
        return self._get_obs()


def apply_walking(
    nao: Nao,
    parameters: Parameters,
    state: State,
    measurements: Measurements,
    control: Control,
    dt: float,
) -> None:
    state, left_sole, left_lift, right_sole, right_lift = walking_engine.step(
        state,
        measurements,
        control,
        dt,
        parameters,
    )

    lower_body_joints = walking_engine.compute_lower_body_joints(
        left_sole,
        right_sole,
        left_lift,
        right_lift,
    )
    left_arm, right_arm = walking_engine.compute_arm_joints(
        left_sole,
        right_sole,
        pitch_factor=parameters.arm_pitch_factor,
    )

    nao.actuators.left_leg.ankle_pitch += lower_body_joints.left.ankle_pitch
    nao.actuators.left_leg.ankle_roll += lower_body_joints.left.ankle_roll
    nao.actuators.left_leg.knee_pitch += lower_body_joints.left.knee_pitch
    nao.actuators.left_leg.hip_pitch += (
        lower_body_joints.left.hip_pitch - parameters.torso_tilt
    )
    nao.actuators.left_leg.hip_roll += lower_body_joints.left.hip_roll
    nao.actuators.left_leg.hip_yaw_pitch += lower_body_joints.left.hip_yaw_pitch

    nao.actuators.right_leg.ankle_pitch += lower_body_joints.right.ankle_pitch
    nao.actuators.right_leg.ankle_roll += lower_body_joints.right.ankle_roll
    nao.actuators.right_leg.knee_pitch += lower_body_joints.right.knee_pitch
    nao.actuators.right_leg.hip_pitch += (
        lower_body_joints.right.hip_pitch - parameters.torso_tilt
    )
    nao.actuators.right_leg.hip_roll += lower_body_joints.right.hip_roll

    nao.actuators.left_arm.shoulder_pitch += left_arm.shoulder_pitch
    nao.actuators.left_arm.shoulder_roll += left_arm.shoulder_roll
    nao.actuators.left_arm.elbow_yaw += left_arm.elbow_yaw
    nao.actuators.left_arm.elbow_roll += left_arm.elbow_roll
    nao.actuators.left_arm.wrist_yaw += left_arm.wrist_yaw

    nao.actuators.right_arm.shoulder_pitch += right_arm.shoulder_pitch
    nao.actuators.right_arm.shoulder_roll += right_arm.shoulder_roll
    nao.actuators.right_arm.elbow_yaw += right_arm.elbow_yaw
    nao.actuators.right_arm.elbow_roll += right_arm.elbow_roll
    nao.actuators.right_arm.wrist_yaw += right_arm.wrist_yaw
