use std::{f32::consts::FRAC_PI_2, time::Duration};

use color_eyre::Result;
use coordinate_systems::Walk;
use linear_algebra::Point3;
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;
use splines::Interpolate;
use types::{
    joints::{arm::ArmJoints, body::BodyJoints, leg::LegJoints},
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

    pub fn compute_joints(
        &self,
        context: &CycleContext,
        same_leg: LegJoints<f32>,
        opposite_foot: Point3<Walk>,
    ) -> ArmJoints<f32> {
        let parameters = &context.parameters.swinging_arms;
        let swinging_arm = swinging_arm(parameters, same_leg, opposite_foot);

        match &self {
            Self::Swing => swinging_arm,
            Self::PullingBack {
                elapsed,
                end_positions,
            } => {
                let interpolation =
                    elapsed.as_secs_f32() / parameters.pulling_back_duration.as_secs_f32();
                ArmJoints::lerp(interpolation, swinging_arm, *end_positions)
            }
            Self::PullingTight { interpolator } => interpolator.value(),
            Self::Back => parameters.pull_tight_joints,
            Self::ReleasingTight { interpolator } => interpolator.value(),
            Self::ReleasingBack {
                elapsed,
                start_positions,
            } => {
                let interpolation =
                    elapsed.as_secs_f32() / parameters.pulling_back_duration.as_secs_f32();
                ArmJoints::lerp(interpolation, *start_positions, swinging_arm)
            }
        }
    }
}

fn swinging_arm(
    parameters: &Parameters,
    same_leg: LegJoints<f32>,
    opposite_foot: Point3<Walk>,
) -> ArmJoints<f32> {
    let shoulder_roll = parameters.default_roll + parameters.roll_factor * same_leg.hip_roll;
    let shoulder_pitch = FRAC_PI_2 - opposite_foot.x() * parameters.pitch_factor;
    ArmJoints {
        shoulder_pitch,
        shoulder_roll,
        elbow_yaw: 0.0,
        elbow_roll: 0.0,
        wrist_yaw: -FRAC_PI_2,
        hand: 0.0,
    }
}

pub trait CompensateArmPullBack {
    fn compensate_arm_pull_back(self, arm: &Arm, parameters: &Parameters) -> BodyJoints<f32>;
}

impl CompensateArmPullBack for BodyJoints<f32> {
    fn compensate_arm_pull_back(self, arm: &Arm, parameters: &Parameters) -> BodyJoints<f32> {
        let shoulder_pitch = match &arm {
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
        };
        let compensation = (shoulder_pitch - FRAC_PI_2) * parameters.torso_tilt_compensation_factor;
        BodyJoints {
            left_leg: LegJoints {
                hip_pitch: self.left_leg.hip_pitch + compensation,
                ..self.left_leg
            },
            right_leg: LegJoints {
                hip_pitch: self.right_leg.hip_pitch + compensation,
                ..self.right_leg
            },
            ..self
        }
    }
}
