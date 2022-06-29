use std::f32::consts::PI;

use anyhow::Result;
use macros::{module, require_some};
use nalgebra::Isometry3;

use crate::types::{
    HeadJoints, HeadJointsCommand, HeadMotion as HeadMotionCommand, MotionCommand, MotionSelection,
    SensorData,
};

pub struct HeadMotion {
    last_request: HeadJoints,
}

#[module(control)]
#[input(path = motion_selection, data_type = MotionSelection)]
#[input(path = motion_command, data_type = MotionCommand)]
#[input(path = sensor_data, data_type = SensorData)]
#[input(path = robot_to_ground, data_type = Isometry3<f32>)]
#[input(path = look_around, data_type = HeadJoints)]
#[input(path = look_at, data_type = HeadJoints)]
#[parameter(path = control.center_head_position, data_type = HeadJoints)]
#[parameter(path = control.head_motion.maximum_velocity, data_type = HeadJoints)]
#[parameter(path = control.head_motion.outer_maximum_pitch, data_type = f32)]
#[parameter(path = control.head_motion.inner_maximum_pitch, data_type = f32)]
#[parameter(path = control.head_motion.outer_yaw, data_type = f32)]
#[main_output(name = head_joints_command, data_type = HeadJointsCommand)]
impl HeadMotion {}

impl HeadMotion {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {
            last_request: Default::default(),
        })
    }

    fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let motion_command = require_some!(context.motion_command);
        let sensor_data = require_some!(context.sensor_data);
        let look_around = require_some!(context.look_around);
        let look_at = require_some!(context.look_at);

        let current_head_angles = sensor_data.positions.head;

        let raw_request = match motion_command {
            MotionCommand::FallProtection { .. } => Default::default(),
            MotionCommand::Jump { .. } => todo!(),
            MotionCommand::Kick { .. } => todo!(),
            MotionCommand::Penalized => Default::default(),
            MotionCommand::StandUp { .. } => Default::default(),
            MotionCommand::Unstiff => current_head_angles,
            MotionCommand::SitDown { head }
            | MotionCommand::Stand { head }
            | MotionCommand::Walk { head, .. }
            | MotionCommand::InWalkKick { head, .. } => match head {
                HeadMotionCommand::ZeroAngles => Default::default(),
                HeadMotionCommand::Center => *context.center_head_position,
                HeadMotionCommand::LookAround => *look_around,
                HeadMotionCommand::LookAt { .. } => *look_at,
                HeadMotionCommand::Unstiff => current_head_angles,
            },
        };
        let maximum_movement =
            *context.maximum_velocity * sensor_data.cycle_info.last_cycle_duration.as_secs_f32();

        let controlled_request = HeadJoints {
            yaw: self.last_request.yaw
                + (raw_request.yaw - self.last_request.yaw)
                    .clamp(-maximum_movement.yaw, maximum_movement.yaw),
            pitch: self.last_request.pitch
                + (raw_request.pitch - self.last_request.pitch)
                    .clamp(-maximum_movement.pitch, maximum_movement.pitch),
        };

        let pitch_max = if controlled_request.yaw.abs() > *context.outer_yaw {
            *context.outer_maximum_pitch
        } else {
            let interpolation_factor =
                0.5 * (1.0 + (PI / *context.outer_yaw * controlled_request.yaw).cos());
            *context.outer_maximum_pitch
                + interpolation_factor
                    * (*context.inner_maximum_pitch - *context.outer_maximum_pitch)
        };
        let clamped_pitch = controlled_request.pitch.clamp(0.0, pitch_max);

        let clamped_request = HeadJoints {
            pitch: clamped_pitch,
            yaw: controlled_request.yaw,
        };

        self.last_request = controlled_request;
        Ok(MainOutputs {
            head_joints_command: Some(HeadJointsCommand {
                positions: clamped_request,
                stiffnesses: HeadJoints::fill(0.8),
            }),
        })
    }
}
