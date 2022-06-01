use std::time::Duration;

use approx::relative_eq;

use macros::{module, require_some};

use crate::types::{HeadJoints, HeadMotionSafeExits, HeadMotionType, MotionSelection, SensorData};

use crate::framework::configuration::{HeadMotionLimits, LookAround as LookAroundConfiguration};

#[derive(Debug)]
pub enum Mode {
    Idle,
    Left,
    Right,
}

impl Default for Mode {
    fn default() -> Self {
        Self::Idle
    }
}

pub struct LookAround {
    last_request: HeadJoints,
    mode: Mode,
    yaw_limit: f32,
    waited_at_end: Duration,
}

#[module(control)]
#[input(path = motion_selection, data_type = MotionSelection)]
#[input(path = sensor_data, data_type = SensorData)]
#[parameter(path = control.look_around, data_type = LookAroundConfiguration)]
#[parameter(path = control.head_motion_limits, data_type = HeadMotionLimits)]
#[persistent_state(path = head_motion_safe_exits, data_type = HeadMotionSafeExits)]
#[main_output(name = look_around, data_type = HeadJoints)]
impl LookAround {}

impl LookAround {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {
            last_request: Default::default(),
            mode: Default::default(),
            yaw_limit: Default::default(),
            waited_at_end: Default::default(),
        })
    }

    fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        let motion_selection = require_some!(context.motion_selection);
        let sensor_data = require_some!(context.sensor_data);
        let current_head_angles = sensor_data.positions.head;
        let head_motion_limits = context.head_motion_limits;
        let configuration = context.look_around;

        let default_output = Ok(MainOutputs {
            look_around: Some(current_head_angles),
        });

        if motion_selection.current_head_motion != HeadMotionType::LookAround {
            self.last_request = current_head_angles;
            return default_output;
        }

        let maximum_yaw_movement_next_cycle = configuration.maximum_yaw_velocity
            * sensor_data.cycle_info.last_cycle_duration.as_secs_f32();
        let maximum_pitch_movement_next_cycle = configuration.maximum_pitch_velocity
            * sensor_data.cycle_info.last_cycle_duration.as_secs_f32();

        self.yaw_limit = match motion_selection.dispatching_head_motion {
            Some(HeadMotionType::LookAround) => head_motion_limits.maximum_yaw,
            Some(_) => 0.0,
            _ => head_motion_limits.maximum_yaw,
        };
        let yaw = match self.mode {
            Mode::Idle => {
                self.mode = Mode::Left;
                self.yaw_limit
            }
            Mode::Left => self.yaw_limit,
            Mode::Right => -self.yaw_limit,
        };
        let desired_angles = HeadJoints { yaw, pitch: 1.0 };

        if relative_eq!(current_head_angles.yaw, desired_angles.yaw, epsilon = 0.05) {
            self.waited_at_end += sensor_data.cycle_info.last_cycle_duration;
            if self.waited_at_end.as_secs_f32() > 0.1 {
                self.mode = match self.mode {
                    Mode::Idle => Mode::Idle,
                    Mode::Left => Mode::Right,
                    Mode::Right => Mode::Left,
                };
                self.waited_at_end = Duration::default();
            }
        }

        let desired_movement = HeadJoints {
            yaw: desired_angles.yaw - self.last_request.yaw,
            pitch: desired_angles.pitch - self.last_request.pitch,
        };

        let movement_request = HeadJoints {
            yaw: desired_movement.yaw.clamp(
                -maximum_yaw_movement_next_cycle,
                maximum_yaw_movement_next_cycle,
            ),
            pitch: desired_movement.pitch.clamp(
                -maximum_pitch_movement_next_cycle,
                maximum_pitch_movement_next_cycle,
            ),
        };
        let request = self.last_request + movement_request;
        self.last_request = request;

        context.head_motion_safe_exits[HeadMotionType::LookAround] = true;

        Ok(MainOutputs {
            look_around: Some(request),
        })
    }
}
