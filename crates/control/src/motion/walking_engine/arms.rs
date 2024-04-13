use std::{f32::consts::FRAC_PI_2, time::Duration};

use color_eyre::Result;
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;
use splines::Interpolate;
use types::{
    joints::{arm::ArmJoints, body::BodyJoints, leg::LegJoints, mirror::Mirror},
    motor_commands::MotorCommands,
    walking_engine::SwingingArmsParameters as Parameters,
};

use motionfile::{SplineInterpolator, TimedSpline};

use super::CycleContext;

#[derive(Clone, Debug, Serialize, Deserialize, SerializeHierarchy, Default)]
pub enum Arm {
    #[default]
    Swing,
    PullingBack {
        elapsed: Duration,
        end_positions: ArmJoints<f32>,
    },
    PullingTight {
        interpolator: SplineInterpolator<ArmJoints<f32>>,
    },
    Back,
    ReleasingTight {
        interpolator: SplineInterpolator<ArmJoints<f32>>,
    },
    ReleasingBack {
        elapsed: Duration,
        start_positions: ArmJoints<f32>,
    },
}

impl Arm {
    pub fn swing(self, context: &CycleContext) -> Self {
        let parameters = &context.parameters.swinging_arms;
        let last_cycle_duration = context.cycle_time.last_cycle_duration;

        match self {
            Self::Swing => self,
            Self::PullingBack {
                elapsed,
                end_positions,
            } => Self::ReleasingBack {
                elapsed: parameters.pulling_back_duration - elapsed,
                start_positions: end_positions,
            },
            Self::PullingTight { interpolator } => {
                let interpolator = TimedSpline::try_new_transition_timed(
                    interpolator.value(),
                    parameters.pull_back_joints,
                    interpolator.current_duration(),
                )
                .unwrap()
                .into();
                Self::ReleasingTight { interpolator }
            }
            Self::Back => {
                let interpolator = TimedSpline::try_new_transition_timed(
                    parameters.pull_tight_joints,
                    parameters.pull_back_joints,
                    parameters.pulling_tight_duration,
                )
                .unwrap()
                .into();
                Self::PullingTight { interpolator }
            }
            Self::ReleasingTight { mut interpolator } => {
                interpolator.advance_by(last_cycle_duration);
                if interpolator.is_finished() {
                    Self::ReleasingBack {
                        elapsed: Duration::ZERO,
                        start_positions: interpolator.value(),
                    }
                } else {
                    Self::ReleasingTight { interpolator }
                }
            }
            Self::ReleasingBack {
                elapsed,
                start_positions,
            } => {
                let elapsed = elapsed + last_cycle_duration;
                if elapsed >= parameters.pulling_back_duration {
                    Self::Swing
                } else {
                    Self::ReleasingBack {
                        elapsed,
                        start_positions,
                    }
                }
            }
        }
    }

    pub fn pull_tight(self, context: &CycleContext) -> Self {
        let parameters = &context.parameters.swinging_arms;
        let last_cycle_duration = context.cycle_time.last_cycle_duration;

        match self {
            Self::Swing => Self::PullingBack {
                elapsed: Duration::ZERO,
                end_positions: parameters.pull_back_joints,
            },
            Self::PullingBack {
                elapsed,
                end_positions,
            } => {
                let elapsed = elapsed + last_cycle_duration;
                if elapsed >= parameters.pulling_back_duration {
                    let interpolator = TimedSpline::try_new_transition_timed(
                        parameters.pull_back_joints,
                        parameters.pull_tight_joints,
                        parameters.pulling_tight_duration,
                    )
                    .unwrap()
                    .into();
                    Self::PullingTight { interpolator }
                } else {
                    Self::PullingBack {
                        elapsed,
                        end_positions,
                    }
                }
            }
            Self::PullingTight { mut interpolator } => {
                interpolator.advance_by(last_cycle_duration);
                if interpolator.is_finished() {
                    Self::Back
                } else {
                    Self::PullingTight { interpolator }
                }
            }
            Self::Back => self,
            Self::ReleasingTight { interpolator } => {
                let interpolator = TimedSpline::try_new_transition_timed(
                    interpolator.value(),
                    parameters.pull_tight_joints,
                    interpolator.current_duration(),
                )
                .unwrap()
                .into();
                Self::PullingTight { interpolator }
            }
            Self::ReleasingBack {
                elapsed,
                start_positions,
            } => Self::PullingBack {
                elapsed: parameters.pulling_back_duration - elapsed,
                end_positions: start_positions,
            },
        }
    }

    fn compute_joints(&self, swinging_arm: ArmJoints, parameters: &Parameters) -> ArmJoints {
        match self {
            Arm::Swing => swinging_arm,
            Arm::PullingBack {
                elapsed,
                end_positions,
            } => {
                let interpolation =
                    elapsed.as_secs_f32() / parameters.pulling_back_duration.as_secs_f32();
                ArmJoints::lerp(interpolation, swinging_arm, *end_positions)
            }
            Arm::PullingTight { interpolator } => interpolator.value(),
            Arm::Back => parameters.pull_tight_joints,
            Arm::ReleasingTight { interpolator } => interpolator.value(),
            Arm::ReleasingBack {
                elapsed,
                start_positions,
            } => {
                let interpolation =
                    elapsed.as_secs_f32() / parameters.pulling_back_duration.as_secs_f32();
                ArmJoints::lerp(interpolation, *start_positions, swinging_arm)
            }
        }
    }

    fn shoulder_pitch(&self, parameters: &Parameters) -> f32 {
        match self {
            Arm::Swing => FRAC_PI_2,
            Arm::PullingBack {
                elapsed,
                end_positions,
            } => {
                let interpolation =
                    elapsed.as_secs_f32() / parameters.pulling_back_duration.as_secs_f32();
                f32::lerp(interpolation, FRAC_PI_2, end_positions.shoulder_pitch)
            }
            Arm::ReleasingBack {
                elapsed,
                start_positions,
            } => {
                let interpolation =
                    elapsed.as_secs_f32() / parameters.pulling_back_duration.as_secs_f32();
                f32::lerp(interpolation, start_positions.shoulder_pitch, FRAC_PI_2)
            }
            Arm::ReleasingTight { interpolator } | Arm::PullingTight { interpolator } => {
                interpolator.value().shoulder_pitch
            }
            Arm::Back => parameters.pull_tight_joints.shoulder_pitch,
        }
    }
}

pub trait ArmOverrides {
    fn override_with_arms(self, parameters: &Parameters, left_arm: &Arm, right_arm: &Arm) -> Self;
}

impl ArmOverrides for MotorCommands<BodyJoints> {
    fn override_with_arms(self, parameters: &Parameters, left_arm: &Arm, right_arm: &Arm) -> Self {
        let left_swinging_arm = self.positions.left_arm;
        let right_swinging_arm = self.positions.right_arm;
        let left_positions = left_arm.compute_joints(left_swinging_arm, parameters);
        let right_positions = right_arm
            .compute_joints(right_swinging_arm.mirrored(), parameters)
            .mirrored();

        let left_compensation = (left_arm.shoulder_pitch(parameters) - FRAC_PI_2)
            * parameters.torso_tilt_compensation_factor;
        let right_compensation = (right_arm.shoulder_pitch(parameters) - FRAC_PI_2)
            * parameters.torso_tilt_compensation_factor;
        let hip_pitch_compensation = left_compensation + right_compensation;

        let positions = BodyJoints {
            left_arm: left_positions,
            right_arm: right_positions,
            left_leg: LegJoints {
                hip_pitch: self.positions.left_leg.hip_pitch + hip_pitch_compensation,
                ..self.positions.left_leg
            },
            right_leg: LegJoints {
                hip_pitch: self.positions.right_leg.hip_pitch + hip_pitch_compensation,
                ..self.positions.right_leg
            },
        };

        Self { positions, ..self }
    }
}
