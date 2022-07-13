use std::time::{Duration, SystemTime, UNIX_EPOCH};

use module_derive::module;
use types::{HeadJoints, HeadMotion, MotionCommand, SensorData, Side};

use crate::framework::configuration;

#[derive(Debug, Clone, Copy)]
pub enum Mode {
    Center { moving_towards: Side },
    Left,
    Right,
    HalfwayLeft { moving_towards: Side },
    HalfwayRight { moving_towards: Side },
}

impl Default for Mode {
    fn default() -> Self {
        Self::Center {
            moving_towards: Side::Left,
        }
    }
}

pub struct LookAround {
    current_mode: Mode,
    last_mode_switch: SystemTime,
}

#[module(control)]
#[input(path = sensor_data, data_type = SensorData, required)]
#[input(path = motion_command, data_type = MotionCommand, required)]
#[parameter(path = control.look_around, data_type = configuration::LookAround, name = config)]
#[main_output(name = look_around, data_type = HeadJoints)]
impl LookAround {}

impl LookAround {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {
            current_mode: Default::default(),
            last_mode_switch: UNIX_EPOCH,
        })
    }

    fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        match context.motion_command.head_motion() {
            Some(HeadMotion::LookAround) => {
                self.look_around(
                    context.sensor_data.cycle_info.start_time,
                    context.config.look_around_timeout,
                );
            }
            Some(HeadMotion::SearchForLostBall) => self.quick_search(
                context.sensor_data.cycle_info.start_time,
                context.config.quick_search_timeout,
            ),
            _ => {
                self.current_mode = Mode::Center {
                    moving_towards: Side::Left,
                };
                return Ok(MainOutputs {
                    look_around: Some(context.config.middle_positions),
                });
            }
        }

        let request = match self.current_mode {
            Mode::Center { .. } => context.config.middle_positions,
            Mode::Left => context.config.left_positions,
            Mode::Right => context.config.right_positions,
            Mode::HalfwayLeft { .. } => context.config.halfway_left_positions,
            Mode::HalfwayRight { .. } => context.config.halfway_right_positions,
        };

        Ok(MainOutputs {
            look_around: Some(request),
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
