use std::time::{Duration, SystemTime, UNIX_EPOCH};

use macros::{module, require_some};

use crate::types::{HeadJoints, SensorData, Side};

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
#[input(path = sensor_data, data_type = SensorData)]
#[parameter(path = control.look_around.time_at_each_position, data_type = Duration)]
#[parameter(path = control.look_around.middle_positions, data_type = HeadJoints)]
#[parameter(path = control.look_around.left_positions, data_type = HeadJoints)]
#[parameter(path = control.look_around.right_positions, data_type = HeadJoints)]
#[parameter(path = control.look_around.halfway_left_positions, data_type = HeadJoints)]
#[parameter(path = control.look_around.halfway_right_positions, data_type = HeadJoints)]
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
        let sensor_data = require_some!(context.sensor_data);

        self.transition(
            sensor_data.cycle_info.start_time,
            *context.time_at_each_position,
        );

        let request = match self.current_mode {
            Mode::Center { .. } => *context.middle_positions,
            Mode::Left => *context.left_positions,
            Mode::Right => *context.right_positions,
            Mode::HalfwayLeft { .. } => *context.halfway_left_positions,
            Mode::HalfwayRight { .. } => *context.halfway_right_positions,
        };

        Ok(MainOutputs {
            look_around: Some(request),
        })
    }

    fn transition(&mut self, start_time: SystemTime, time_at_each_position: Duration) {
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
}
