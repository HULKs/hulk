use std::{f32::consts::FRAC_PI_2, time::Duration};

use color_eyre::Result;
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;
use types::{
    joints::ArmJoints,
    motion_command::{ArmMotion, MotionCommand},
    parameters::SwingingArmsParameters,
    support_foot::Side,
};

use motionfile::{SplineInterpolator, TimedSpline};

use super::foot_offsets::FootOffsets;

#[derive(Clone, Default, Debug, Serialize, Deserialize, SerializeHierarchy)]
pub struct SwingingArm {
    side: Side,
    state: State,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize, SerializeHierarchy)]
enum State {
    #[default]
    Swing,
    PullingBack {
        interpolator: SplineInterpolator<ArmJoints<f32>>,
    },
    PullingTight {
        interpolator: SplineInterpolator<ArmJoints<f32>>,
    },
    Back,
    ReleasingTight {
        interpolator: SplineInterpolator<ArmJoints<f32>>,
    },
    ReleasingBack {
        interpolator: SplineInterpolator<ArmJoints<f32>>,
    },
}

impl SwingingArm {
    pub fn new(side: Side) -> Self {
        Self {
            side,
            state: State::Swing,
        }
    }

    pub fn next(
        &mut self,
        foot: FootOffsets,
        motion_command: &MotionCommand,
        cycle_duration: Duration,
        config: &SwingingArmsParameters,
    ) -> Result<ArmJoints<f32>> {
        let requested_arm_motion =
            self.arm_motion_from_motion_command(motion_command, config.debug_pull_back);
        let pull_back_joints = match self.side {
            Side::Left => config.pull_back_joints,
            Side::Right => config.pull_back_joints.mirrored(),
        };
        let pull_tight_joints = match self.side {
            Side::Left => config.pull_tight_joints,
            Side::Right => config.pull_tight_joints.mirrored(),
        };
        let swinging_arm_joints = self.swinging_arm_joints(foot, config);
        let center_arm_joints = self.swinging_arm_joints(FootOffsets::zero(), config);

        self.state = match (&mut self.state, requested_arm_motion) {
            (State::Swing, ArmMotion::Swing) => State::Swing,
            (State::Swing, ArmMotion::PullTight) => State::PullingBack {
                interpolator: TimedSpline::try_new_transition_timed(
                    swinging_arm_joints,
                    pull_back_joints,
                    config.pulling_back_duration,
                )?
                .into(),
            },
            (
                State::PullingBack {
                    ref mut interpolator,
                },
                ArmMotion::PullTight,
            ) => {
                interpolator.advance_by(cycle_duration);
                if interpolator.is_finished() {
                    State::PullingTight {
                        interpolator: TimedSpline::try_new_transition_timed(
                            pull_back_joints,
                            pull_tight_joints,
                            config.pulling_tight_duration,
                        )?
                        .into(),
                    }
                } else {
                    State::PullingBack {
                        interpolator: interpolator.clone(),
                    }
                }
            }
            (State::PullingBack { interpolator }, ArmMotion::Swing) => {
                let current_joints = interpolator.value();
                let interpolator = TimedSpline::try_new_transition_timed(
                    current_joints,
                    center_arm_joints,
                    interpolator.current_duration(),
                )?;
                State::ReleasingBack {
                    interpolator: interpolator.into(),
                }
            }
            (
                State::PullingTight {
                    ref mut interpolator,
                },
                ArmMotion::PullTight,
            ) => {
                interpolator.advance_by(cycle_duration);
                if interpolator.is_finished() {
                    State::Back
                } else {
                    State::PullingTight {
                        interpolator: interpolator.clone(),
                    }
                }
            }
            (State::PullingTight { interpolator }, ArmMotion::Swing) => {
                let current_joints = interpolator.value();
                let interpolator = TimedSpline::try_new_transition_timed(
                    current_joints,
                    pull_back_joints,
                    interpolator.current_duration(),
                )?
                .into();
                State::ReleasingTight { interpolator }
            }
            (State::Back, ArmMotion::Swing) => State::ReleasingTight {
                interpolator: TimedSpline::try_new_transition_timed(
                    pull_tight_joints,
                    pull_back_joints,
                    config.pulling_back_duration + config.pulling_tight_duration,
                )?
                .into(),
            },
            (State::Back, ArmMotion::PullTight) => State::Back,
            (
                State::ReleasingBack {
                    ref mut interpolator,
                },
                ArmMotion::Swing,
            ) => {
                interpolator.advance_by(cycle_duration);
                if interpolator.is_finished() {
                    State::Swing
                } else {
                    State::ReleasingBack {
                        interpolator: interpolator.clone(),
                    }
                }
            }
            (State::ReleasingBack { interpolator }, ArmMotion::PullTight) => {
                let current_joints = interpolator.value();
                let interpolator = TimedSpline::try_new_transition_timed(
                    current_joints,
                    pull_back_joints,
                    config.pulling_back_duration,
                )?;
                State::PullingBack {
                    interpolator: interpolator.into(),
                }
            }
            (
                State::ReleasingTight {
                    ref mut interpolator,
                },
                ArmMotion::Swing,
            ) => {
                interpolator.advance_by(cycle_duration);
                if interpolator.is_finished() {
                    State::ReleasingBack {
                        interpolator: TimedSpline::try_new_transition_timed(
                            pull_back_joints,
                            center_arm_joints,
                            config.pulling_back_duration,
                        )?
                        .into(),
                    }
                } else {
                    State::ReleasingTight {
                        interpolator: interpolator.clone(),
                    }
                }
            }
            (State::ReleasingTight { interpolator }, ArmMotion::PullTight) => {
                let current_joints = interpolator.value();
                let interpolator = TimedSpline::try_new_transition_timed(
                    current_joints,
                    pull_tight_joints,
                    interpolator.current_duration(),
                )?;
                State::PullingTight {
                    interpolator: interpolator.into(),
                }
            }
        };
        Ok(match &self.state {
            State::Swing => swinging_arm_joints,
            State::PullingBack { interpolator }
            | State::ReleasingBack { interpolator }
            | State::ReleasingTight { interpolator }
            | State::PullingTight { interpolator } => interpolator.value(),
            State::Back => pull_tight_joints,
        })
    }

    pub fn torso_tilt_compensation(&self, config: &SwingingArmsParameters) -> Result<f32> {
        let shoulder_pitch = match &self.state {
            State::Swing => FRAC_PI_2,
            State::PullingBack { interpolator }
            | State::ReleasingBack { interpolator }
            | State::ReleasingTight { interpolator }
            | State::PullingTight { interpolator } => interpolator.value().shoulder_pitch,
            State::Back => config.pull_tight_joints.shoulder_pitch,
        };
        Ok((shoulder_pitch - FRAC_PI_2) * config.torso_tilt_compensation_factor)
    }

    fn arm_motion_from_motion_command(
        &self,
        motion_command: &MotionCommand,
        debug_pull_back: bool,
    ) -> ArmMotion {
        if debug_pull_back {
            return ArmMotion::PullTight;
        }
        match motion_command {
            MotionCommand::Walk {
                left_arm,
                right_arm,
                ..
            } => match self.side {
                Side::Left => *left_arm,
                Side::Right => *right_arm,
            },
            _ => ArmMotion::Swing,
        }
    }

    fn swinging_arm_joints(
        &self,
        foot: FootOffsets,
        config: &SwingingArmsParameters,
    ) -> ArmJoints<f32> {
        let shoulder_roll = config.default_roll + config.roll_factor * foot.left.abs();
        let shoulder_pitch = FRAC_PI_2 + foot.forward * config.pitch_factor;
        let joints = ArmJoints {
            shoulder_pitch,
            shoulder_roll,
            elbow_yaw: -FRAC_PI_2,
            elbow_roll: 0.0,
            wrist_yaw: 0.0,
            hand: 0.0,
        };
        match self.side {
            Side::Left => joints,
            Side::Right => joints.mirrored(),
        }
    }
}
