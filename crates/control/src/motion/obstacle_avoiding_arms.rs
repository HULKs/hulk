use std::time::Duration;

use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use motionfile::{SplineInterpolator, TimedSpline};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;
use types::{
    cycle_time::CycleTime,
    joints::{arm::ArmJoints, mirror::Mirror as _},
    motion_command::{ArmMotion, MotionCommand},
    obstacle_avoiding_arms::{ArmCommand, ArmCommands},
    support_foot::Side,
};
use walking_engine::parameters::SwingingArmsParameters;

#[derive(Deserialize, Serialize)]
pub struct ObstacleAvoidingArms {
    pub left_arm: Option<Arm>,
    pub right_arm: Option<Arm>,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    motion_command: Input<MotionCommand, "motion_command">,
    cycle_time: Input<CycleTime, "cycle_time">,
    parameters: Parameter<SwingingArmsParameters, "walking_engine.swinging_arms">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub obstacle_avoiding_arms: MainOutput<ArmCommands>,
}

impl ObstacleAvoidingArms {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            left_arm: Some(Arm::Swing),
            right_arm: Some(Arm::Swing),
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        // enter the functional world...
        let left_arm = self.left_arm.take().unwrap();
        let right_arm = self.right_arm.take().unwrap();

        let left_arm = match context.motion_command.arm_motion(Side::Left) {
            Some(ArmMotion::Swing) | None => left_arm.swing(&context),
            Some(ArmMotion::PullTight) => left_arm.pull_tight(&context),
        };
        let right_arm = match context.motion_command.arm_motion(Side::Right) {
            Some(ArmMotion::Swing) | None => right_arm.swing(&context),
            Some(ArmMotion::PullTight) => right_arm.pull_tight(&context),
        };
        // do not forget to put it back ;)
        self.left_arm = Some(left_arm);
        self.right_arm = Some(right_arm);

        Ok(MainOutputs {
            obstacle_avoiding_arms: ArmCommands {
                left_arm: self
                    .left_arm
                    .as_ref()
                    .unwrap()
                    .to_command(context.parameters),
                right_arm: self
                    .right_arm
                    .as_ref()
                    .unwrap()
                    .to_command(context.parameters)
                    .mirrored(),
            }
            .into(),
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, SerializeHierarchy, Default)]
pub enum Arm {
    #[default]
    Swing,
    PullingBack {
        elapsed: Duration,
        end_positions: ArmJoints<f32>,
    },
    PullingTight {
        interpolator: SplineInterpolator<ArmJoints>,
    },
    Tight,
    ReleasingTight {
        interpolator: SplineInterpolator<ArmJoints>,
    },
    ReleasingBack {
        elapsed: Duration,
        start_positions: ArmJoints,
    },
}

impl Arm {
    pub fn swing(self, context: &CycleContext) -> Self {
        let parameters = &context.parameters;
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
            Self::Tight => {
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
        let parameters = &context.parameters;
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
                    Self::Tight
                } else {
                    Self::PullingTight { interpolator }
                }
            }
            Self::Tight => self,
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

    fn to_command(&self, parameters: &SwingingArmsParameters) -> ArmCommand {
        match self {
            Arm::Swing => ArmCommand::Swing,
            Arm::PullingBack {
                elapsed,
                end_positions,
            } => {
                let interpolation =
                    elapsed.as_secs_f32() / parameters.pulling_back_duration.as_secs_f32();
                ArmCommand::Activating {
                    influence: interpolation,
                    positions: *end_positions,
                }
            }
            Arm::PullingTight { interpolator } => ArmCommand::Active {
                positions: interpolator.value(),
            },
            Arm::Tight => ArmCommand::Active {
                positions: parameters.pull_tight_joints,
            },
            Arm::ReleasingTight { interpolator } => ArmCommand::Active {
                positions: interpolator.value(),
            },
            Arm::ReleasingBack {
                elapsed,
                start_positions,
            } => {
                let interpolation =
                    elapsed.as_secs_f32() / parameters.pulling_back_duration.as_secs_f32();
                ArmCommand::Activating {
                    influence: 1.0 - interpolation,
                    positions: *start_positions,
                }
            }
        }
    }
}
