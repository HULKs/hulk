from typing import Any, override

import numpy as np
import walking_engine
from gymnasium import utils
from mujoco_interactive_viewer.context import current_viewer
from nao_interface import READY_POSE, Nao
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
from rewards.walk_rewards import swing_sole_to_target
from transforms.transforms import (
    Pose2,
)
from walking_engine import (
    Control,
    Measurements,
    Parameters,
)
from walking_engine.walking_types import Feet, Side, State

from .nao_base_env import NaoBaseEnv

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


class NaoWalking(NaoBaseEnv, utils.EzPickle):
    def __init__(self, *, throw_tomatoes: bool, **kwargs: Any) -> None:
        super().__init__(
            throw_tomatoes=throw_tomatoes,
            sensor_delay=3,
            **kwargs,
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
            arm_pitch_factor=8.0,
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

    @override
    def step(
        self,
        action: NDArray[np.floating],
    ) -> tuple[
        NDArray[np.float64],
        np.float64,
        bool,
        bool,
        dict[str, np.float64],
    ]:
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
            RewardContext(self.nao, action, self.state)
        )
        reward = sum(distinct_rewards.values(), np.float64(0.0))

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
        measurements = Measurements(
            pressure_left=self.nao.left_fsr().sum(),
            pressure_right=self.nao.right_fsr().sum(),
        )
        self.nao.actuator_control.set_from_joints(READY_POSE)

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
                self.nao,
                parameters=self.parameters,
                state=self.state,
                measurements=measurements,
                control=control,
                dt=dt,
            )
        current_control = self.nao.actuator_control.to_numpy(
            self.actuator_names
        )
        super().do_simulation(current_control + ctrl, self.frame_skip)

    @override
    def reset_model(self) -> NDArray[np.floating]:
        self.current_step = 0
        self.state = initial_state(self.parameters)
        self.reward.reset()
        self.set_state(
            self.init_qpos,
            self.init_qvel,
        )
        self.nao.reset(READY_POSE)

        measurements = Measurements(
            self.nao.left_fsr().sum(),
            self.nao.right_fsr().sum(),
        )

        apply_walking(
            self.nao,
            self.parameters,
            self.state,
            measurements,
            Control(0.0, 0.0, 0.0),
            0.0,
        )

        self.enable_walking = False
        action_space_size = self.action_space.shape[0]
        self.do_simulation(
            np.zeros(action_space_size),
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

    if (viewer := current_viewer()) is not None:
        current_swing_sole_to_target = swing_sole_to_target(
            nao.data.site("left_sole").xpos.copy(),
            nao.data.site("right_sole").xpos.copy(),
            state,
        )
        swing_sole = (
            nao.data.site("left_sole").xpos
            if state.support_side == Side.RIGHT
            else nao.data.site("right_sole").xpos
        )
        viewer.mark_sphere(
            position=swing_sole - current_swing_sole_to_target,
            radius=0.01,
            rgba=[0.5, 0, 0, 0.3],
        )
        viewer.mark_arrow(
            position=swing_sole,
            direction=-current_swing_sole_to_target,
            width=0.01,
            rgba=[0.5, 0, 0, 0.3],
        )

    legs = walking_engine.compute_lower_body_joints(
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

    actuator = nao.actuator_control
    actuator.hip_yaw_pitch = legs.left.hip_yaw_pitch

    actuator.left_arm.shoulder_pitch = left_arm.shoulder_pitch
    actuator.left_arm.shoulder_roll = left_arm.shoulder_roll
    actuator.left_arm.elbow_yaw = left_arm.elbow_yaw
    actuator.left_arm.elbow_roll = left_arm.elbow_roll
    actuator.left_arm.wrist_yaw = left_arm.wrist_yaw

    actuator.right_arm.shoulder_pitch = right_arm.shoulder_pitch
    actuator.right_arm.shoulder_roll = right_arm.shoulder_roll
    actuator.right_arm.elbow_yaw = right_arm.elbow_yaw
    actuator.right_arm.elbow_roll = right_arm.elbow_roll
    actuator.right_arm.wrist_yaw = right_arm.wrist_yaw

    actuator.left_leg.ankle_pitch = legs.left.ankle_pitch
    actuator.left_leg.ankle_roll = legs.left.ankle_roll
    actuator.left_leg.knee_pitch = legs.left.knee_pitch
    actuator.left_leg.hip_pitch = legs.left.hip_pitch - parameters.torso_tilt
    actuator.left_leg.hip_roll = legs.left.hip_roll

    actuator.right_leg.ankle_pitch = legs.right.ankle_pitch
    actuator.right_leg.ankle_roll = legs.right.ankle_roll
    actuator.right_leg.knee_pitch = legs.right.knee_pitch
    actuator.right_leg.hip_pitch = legs.right.hip_pitch - parameters.torso_tilt
    actuator.right_leg.hip_roll = legs.right.hip_roll
