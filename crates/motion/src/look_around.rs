use std::time::{Duration, SystemTime, UNIX_EPOCH};

use color_eyre::Result;
use context_attribute::context;
use framework::{AdditionalOutput, MainOutput};
use serde::{Deserialize, Serialize};
use types::{
    cycle_time::CycleTime,
    initial_look_around::{
        BallSearchLookAround, InitialLookAround, LookAroundMode, QuickLookAround,
    },
    joints::head::HeadJoints,
    motion_command::{HeadMotion, MotionCommand},
    parameters::LookAroundParameters,
    support_foot::Side,
};

#[derive(Deserialize, Serialize)]
pub struct LookAround {
    current_mode: LookAroundMode,
    last_mode_switch: SystemTime,
    last_head_motion: Option<HeadMotion>,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    config: Parameter<LookAroundParameters, "look_around">,
    motion_command: Input<MotionCommand, "selected_motion_command">,
    cycle_time: Input<CycleTime, "cycle_time">,
    current_mode: AdditionalOutput<LookAroundMode, "look_around_mode">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub look_around_target_joints: MainOutput<HeadJoints<f32>>,
}

impl LookAround {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            current_mode: LookAroundMode::Initial(InitialLookAround::default()),
            last_mode_switch: UNIX_EPOCH,
            last_head_motion: None,
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        if self.last_head_motion != context.motion_command.head_motion() {
            self.last_mode_switch = context.cycle_time.start_time;
            self.current_mode = match context.motion_command.head_motion() {
                Some(HeadMotion::SearchForLostBall) => {
                    LookAroundMode::BallSearch(Default::default())
                }
                Some(HeadMotion::LookAround) => LookAroundMode::QuickSearch(Default::default()),
                _ => LookAroundMode::Center,
            };
        }

        self.last_head_motion = context.motion_command.head_motion();

        match context.motion_command.head_motion() {
            Some(HeadMotion::LookAround) => {
                self.look_around(
                    context.cycle_time.start_time,
                    context.config.look_around_timeout,
                );
            }
            Some(HeadMotion::SearchForLostBall) => self.look_around(
                context.cycle_time.start_time,
                context.config.quick_search_timeout,
            ),
            _ => {
                self.current_mode = LookAroundMode::Center;
                context
                    .current_mode
                    .fill_if_subscribed(|| self.current_mode);
                return Ok(MainOutputs {
                    look_around_target_joints: HeadJoints::fill(0.0).into(),
                });
            }
        }

        context
            .current_mode
            .fill_if_subscribed(|| self.current_mode);

        let request = match self.current_mode {
            LookAroundMode::Center => context.config.middle_positions,
            LookAroundMode::QuickSearch(QuickLookAround { mode: state })
            | LookAroundMode::BallSearch(state) => match state {
                BallSearchLookAround::Center { .. } => context.config.middle_positions,
                BallSearchLookAround::Left => context.config.left_positions,
                BallSearchLookAround::Right => context.config.right_positions,
                BallSearchLookAround::HalfwayLeft { .. } => context.config.halfway_left_positions,
                BallSearchLookAround::HalfwayRight { .. } => context.config.halfway_right_positions,
            },
            LookAroundMode::Initial(state) => match state {
                InitialLookAround::Left => context.config.initial_left_positions,
                InitialLookAround::Right => context.config.initial_right_positions,
            },
        };

        Ok(MainOutputs {
            look_around_target_joints: request.into(),
        })
    }

    fn look_around(&mut self, start_time: SystemTime, time_at_each_position: Duration) {
        if start_time.duration_since(self.last_mode_switch).unwrap() < time_at_each_position {
            return;
        }
        self.last_mode_switch = start_time;
        self.current_mode = match self.current_mode {
            LookAroundMode::Center => LookAroundMode::Center,
            LookAroundMode::BallSearch(state) => LookAroundMode::BallSearch(state.next()),
            LookAroundMode::QuickSearch(state) => LookAroundMode::QuickSearch(state.next()),
            LookAroundMode::Initial(state) => LookAroundMode::Initial(state.next()),
        }
    }
}

trait NextMode {
    fn next(&self) -> Self;
}

impl NextMode for BallSearchLookAround {
    fn next(&self) -> Self {
        match self {
            BallSearchLookAround::Center {
                moving_towards: Side::Left,
            } => BallSearchLookAround::HalfwayLeft {
                moving_towards: Side::Left,
            },
            BallSearchLookAround::Center {
                moving_towards: Side::Right,
            } => BallSearchLookAround::HalfwayRight {
                moving_towards: Side::Right,
            },
            BallSearchLookAround::Left => BallSearchLookAround::HalfwayLeft {
                moving_towards: Side::Right,
            },
            BallSearchLookAround::Right => BallSearchLookAround::HalfwayRight {
                moving_towards: Side::Left,
            },
            BallSearchLookAround::HalfwayLeft {
                moving_towards: Side::Left,
            } => BallSearchLookAround::Left,
            BallSearchLookAround::HalfwayLeft {
                moving_towards: Side::Right,
            } => BallSearchLookAround::Center {
                moving_towards: Side::Right,
            },
            BallSearchLookAround::HalfwayRight {
                moving_towards: Side::Left,
            } => BallSearchLookAround::Center {
                moving_towards: Side::Left,
            },
            BallSearchLookAround::HalfwayRight {
                moving_towards: Side::Right,
            } => BallSearchLookAround::Right,
        }
    }
}

impl NextMode for InitialLookAround {
    fn next(&self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }
}

impl NextMode for QuickLookAround {
    fn next(&self) -> Self {
        let mode = match self.mode {
            BallSearchLookAround::Center {
                moving_towards: Side::Left,
            } => BallSearchLookAround::HalfwayLeft {
                moving_towards: Side::Right,
            },
            BallSearchLookAround::Center {
                moving_towards: Side::Right,
            } => BallSearchLookAround::HalfwayRight {
                moving_towards: Side::Left,
            },
            BallSearchLookAround::Left => BallSearchLookAround::HalfwayLeft {
                moving_towards: Side::Right,
            },
            BallSearchLookAround::Right => BallSearchLookAround::HalfwayRight {
                moving_towards: Side::Left,
            },
            BallSearchLookAround::HalfwayLeft { .. } => BallSearchLookAround::Center {
                moving_towards: Side::Right,
            },
            BallSearchLookAround::HalfwayRight { .. } => BallSearchLookAround::Center {
                moving_towards: Side::Left,
            },
        };
        Self { mode }
    }
}
