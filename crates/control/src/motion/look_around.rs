use std::time::{Duration, SystemTime, UNIX_EPOCH};

use color_eyre::Result;
use context_attribute::context;
use framework::{AdditionalOutput, MainOutput};
use types::{
    initial_look_around::Mode, parameters::LookAround as LookAroundParameters, CycleTime,
    HeadJoints, HeadMotion, MotionCommand, Side,
};

pub struct LookAround {
    current_mode: Mode,
    last_mode_switch: SystemTime,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    config: Parameter<LookAroundParameters, "look_around">,

    motion_command: Input<MotionCommand, "motion_command">,
    cycle_time: Input<CycleTime, "cycle_time">,
    current_mode: AdditionalOutput<Mode, "look_around_mode">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub look_around: MainOutput<HeadJoints<f32>>,
}

impl LookAround {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            current_mode: Default::default(),
            last_mode_switch: UNIX_EPOCH,
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        match context.motion_command.head_motion() {
            Some(HeadMotion::LookAround) => {
                self.look_around(
                    context.cycle_time.start_time,
                    context.config.look_around_timeout,
                );
            }
            Some(HeadMotion::SearchForLostBall) => self.quick_search(
                context.cycle_time.start_time,
                context.config.quick_search_timeout,
            ),
            _ => {
                self.current_mode = Mode::Center {
                    moving_towards: Side::Left,
                };
                context
                    .current_mode
                    .fill_if_subscribed(|| self.current_mode);
                return Ok(MainOutputs {
                    look_around: context.config.middle_positions.into(),
                });
            }
        }

        context
            .current_mode
            .fill_if_subscribed(|| self.current_mode);

        let request = match self.current_mode {
            Mode::Center { .. } => context.config.middle_positions,
            Mode::Left => context.config.left_positions,
            Mode::Right => context.config.right_positions,
            Mode::HalfwayLeft { .. } => context.config.halfway_left_positions,
            Mode::HalfwayRight { .. } => context.config.halfway_right_positions,
        };

        Ok(MainOutputs {
            look_around: request.into(),
        })
    }

    fn look_around(&mut self, start_time: SystemTime, time_at_each_position: Duration) {
        if start_time.duration_since(self.last_mode_switch).unwrap() < time_at_each_position {
            return;
        }
        self.last_mode_switch = start_time;
        self.current_mode = match self.current_mode {
            Mode::Center {
                moving_towards: Side::Left,
            } => Mode::HalfwayLeft {
                moving_towards: Side::Left,
            },
            Mode::Center {
                moving_towards: Side::Right,
            } => Mode::HalfwayRight {
                moving_towards: Side::Right,
            },
            Mode::Left => Mode::HalfwayLeft {
                moving_towards: Side::Right,
            },
            Mode::Right => Mode::HalfwayRight {
                moving_towards: Side::Left,
            },
            Mode::HalfwayLeft {
                moving_towards: Side::Left,
            } => Mode::Left,
            Mode::HalfwayLeft {
                moving_towards: Side::Right,
            } => Mode::Center {
                moving_towards: Side::Right,
            },
            Mode::HalfwayRight {
                moving_towards: Side::Left,
            } => Mode::Center {
                moving_towards: Side::Left,
            },
            Mode::HalfwayRight {
                moving_towards: Side::Right,
            } => Mode::Right,
        }
    }

    fn quick_search(&mut self, start_time: SystemTime, time_at_each_position: Duration) {
        if start_time.duration_since(self.last_mode_switch).unwrap() < time_at_each_position {
            return;
        }
        self.last_mode_switch = start_time;
        self.current_mode = match self.current_mode {
            Mode::Center {
                moving_towards: Side::Left,
            } => Mode::HalfwayLeft {
                moving_towards: Side::Right,
            },
            Mode::Center {
                moving_towards: Side::Right,
            } => Mode::HalfwayRight {
                moving_towards: Side::Left,
            },
            Mode::Left => Mode::HalfwayLeft {
                moving_towards: Side::Right,
            },
            Mode::Right => Mode::HalfwayRight {
                moving_towards: Side::Left,
            },
            Mode::HalfwayLeft { .. } => Mode::Center {
                moving_towards: Side::Right,
            },
            Mode::HalfwayRight { .. } => Mode::Center {
                moving_towards: Side::Left,
            },
        }
    }
}
