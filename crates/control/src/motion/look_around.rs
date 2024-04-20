use std::time::{Duration, SystemTime, UNIX_EPOCH};

use color_eyre::Result;
use context_attribute::context;
use framework::{AdditionalOutput, MainOutput};
use serde::{Deserialize, Serialize};
use types::{
    cycle_time::CycleTime,
    filtered_game_controller_state::FilteredGameControllerState,
    initial_look_around::Mode,
    joints::head::HeadJoints,
    motion_command::{HeadMotion, MotionCommand},
    motion_selection::MotionSelection,
    parameters::LookAroundParameters,
    support_foot::Side,
};

#[derive(Deserialize, Serialize)]
pub struct LookAround {
    current_mode: Mode,
    last_mode_switch: SystemTime,
    last_head_motion: Option<HeadMotion>,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    config: Parameter<LookAroundParameters, "look_around">,

    filtered_game_controller_state:
        Input<Option<FilteredGameControllerState>, "filtered_game_controller_state?">,
    motion_command: Input<MotionCommand, "motion_command">,
    motion_selection: Input<MotionSelection, "motion_selection">,
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
            last_head_motion: None,
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        if self.last_head_motion != context.motion_command.head_motion()
            || context.motion_selection.dispatching_motion.is_some()
        {
            self.last_mode_switch = context.cycle_time.start_time;
            self.current_mode = match context.motion_command.head_motion() {
                Some(HeadMotion::LookAround) => context.filtered_game_controller_state.map_or(
                    Mode::InitialLeft,
                    |filtered_game_controller_state| {
                        if filtered_game_controller_state.own_team_is_home_after_coin_toss {
                            Mode::InitialLeft
                        } else {
                            Mode::InitialRight
                        }
                    },
                ),
                Some(HeadMotion::SearchForLostBall) => Mode::Center {
                    moving_towards: Side::Left,
                },
                _ => Mode::Center {
                    moving_towards: Side::Left,
                },
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
            Some(HeadMotion::SearchForLostBall) => self.quick_search(
                context.cycle_time.start_time,
                context.config.quick_search_timeout,
            ),
            Some(HeadMotion::ZeroAngles) => {
                return Ok(MainOutputs {
                    look_around: HeadJoints::fill(0.0).into(),
                })
            }
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
            Mode::InitialLeft => context.config.initial_left_positions,
            Mode::InitialRight => context.config.initial_right_positions,
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
            Mode::InitialLeft => Mode::InitialRight,
            Mode::InitialRight => Mode::InitialLeft,

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
            Mode::InitialLeft => Mode::InitialRight,
            Mode::InitialRight => Mode::InitialLeft,
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
