use std::{f32::consts::FRAC_PI_2, time::Duration};

use serde::{Deserialize, Serialize};
use types::{ArmJoints, ArmMotion, MotionCommand, Side};

use crate::{control::linear_interpolator::LinearInterpolator, framework::configuration};

use super::foot_offsets::FootOffsets;

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct SwingingArm {
    side: Side,
    state: State,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
enum State {
    #[default]
    Swing,
    PullingBack {
        interpolator: LinearInterpolator<ArmJoints>,
    },
    PullingTight {
        interpolator: LinearInterpolator<ArmJoints>,
    },
    Back,
    ReleasingTight {
        interpolator: LinearInterpolator<ArmJoints>,
    },
    ReleasingBack {
        interpolator: LinearInterpolator<ArmJoints>,
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
        config: &configuration::SwingingArms,
    ) -> ArmJoints {
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

        self.state = match (&self.state, requested_arm_motion) {
            (State::Swing, ArmMotion::Swing) => State::Swing,
            (State::Swing, ArmMotion::PullTight) => State::PullingBack {
                interpolator: LinearInterpolator::new(
                    swinging_arm_joints,
                    pull_back_joints,
                    config.pulling_back_duration,
                ),
            },
            (State::PullingBack { mut interpolator }, ArmMotion::PullTight) => {
                interpolator.step(cycle_duration);
                if interpolator.is_finished() {
                    State::PullingTight {
                        interpolator: LinearInterpolator::new(
                            pull_back_joints,
                            pull_tight_joints,
                            config.pulling_tight_duration,
                        ),
                    }
                } else {
                    State::PullingBack { interpolator }
                }
            }
            (State::PullingBack { interpolator }, ArmMotion::Swing) => {
                let current_joints = interpolator.value();
                let interpolator = LinearInterpolator::new(
                    current_joints,
                    center_arm_joints,
                    interpolator.passed_duration(),
                );
                State::ReleasingBack { interpolator }
            }
            (State::PullingTight { mut interpolator }, ArmMotion::PullTight) => {
                interpolator.step(cycle_duration);
                if interpolator.is_finished() {
                    State::Back
                } else {
                    State::PullingTight { interpolator }
                }
            }
            (State::PullingTight { interpolator }, ArmMotion::Swing) => {
                let current_joints = interpolator.value();
                let interpolator = LinearInterpolator::new(
                    current_joints,
                    pull_back_joints,
                    interpolator.passed_duration(),
                );
                State::ReleasingTight { interpolator }
            }
            (State::Back, ArmMotion::Swing) => State::ReleasingTight {
                interpolator: LinearInterpolator::new(
                    pull_tight_joints,
                    pull_back_joints,
                    config.pulling_back_duration + config.pulling_tight_duration,
                ),
            },
            (State::Back, ArmMotion::PullTight) => State::Back,
            (State::ReleasingBack { mut interpolator }, ArmMotion::Swing) => {
                interpolator.step(cycle_duration);
                if interpolator.is_finished() {
                    State::Swing
                } else {
                    State::ReleasingBack { interpolator }
                }
            }
            (State::ReleasingBack { interpolator }, ArmMotion::PullTight) => {
                let current_joints = interpolator.value();
                let interpolator = LinearInterpolator::new(
                    current_joints,
                    pull_back_joints,
                    config.pulling_back_duration,
                );
                State::PullingBack { interpolator }
            }
            (State::ReleasingTight { mut interpolator }, ArmMotion::Swing) => {
                interpolator.step(cycle_duration);
                if interpolator.is_finished() {
                    State::ReleasingBack {
                        interpolator: LinearInterpolator::new(
                            pull_back_joints,
                            center_arm_joints,
                            config.pulling_back_duration,
                        ),
                    }
                } else {
                    State::ReleasingTight { interpolator }
                }
            }
            (State::ReleasingTight { interpolator }, ArmMotion::PullTight) => {
                let current_joints = interpolator.value();
                let interpolator = LinearInterpolator::new(
                    current_joints,
                    pull_tight_joints,
                    interpolator.passed_duration(),
                );
                State::PullingTight { interpolator }
            }
        };
        match &self.state {
            State::Swing => swinging_arm_joints,
            State::PullingBack { interpolator }
            | State::ReleasingBack { interpolator }
            | State::ReleasingTight { interpolator }
            | State::PullingTight { interpolator } => interpolator.value(),
            State::Back => pull_tight_joints,
        }
    }

    pub fn torso_tilt_compensation(&self, config: &configuration::SwingingArms) -> f32 {
        let shoulder_pitch = match &self.state {
            State::Swing => FRAC_PI_2,
            State::PullingBack { interpolator }
            | State::ReleasingBack { interpolator }
            | State::ReleasingTight { interpolator }
            | State::PullingTight { interpolator } => interpolator.value().shoulder_pitch,
            State::Back => config.pull_tight_joints.shoulder_pitch,
        };
        (shoulder_pitch - FRAC_PI_2) * config.torso_tilt_compensation_factor
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
        config: &configuration::SwingingArms,
    ) -> ArmJoints {
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
