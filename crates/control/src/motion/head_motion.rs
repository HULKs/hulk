use std::f32::consts::PI;

use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use serde::{Deserialize, Serialize};
use types::{
    cycle_time::CycleTime,
    joints::head::HeadJoints,
    motion_command::{HeadMotion as HeadMotionCommand, MotionCommand},
    motor_commands::HeadMotorCommand,
    sensor_data::SensorData,
};

#[derive(Default, Deserialize, Serialize)]
pub struct HeadMotion {
    last_positions: HeadJoints<f32>,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    center_head_position: Parameter<HeadJoints<f32>, "center_head_position">,
    inner_maximum_pitch: Parameter<f32, "head_motion.inner_maximum_pitch">,
    maximum_velocity: Parameter<HeadJoints<f32>, "head_motion.maximum_velocity">,
    outer_maximum_pitch: Parameter<f32, "head_motion.outer_maximum_pitch">,
    outer_yaw: Parameter<f32, "head_motion.outer_yaw">,

    look_around: Input<HeadJoints<f32>, "look_around">,
    look_at: Input<HeadJoints<f32>, "look_at">,
    motion_command: Input<MotionCommand, "motion_command">,
    sensor_data: Input<SensorData, "sensor_data">,
    cycle_time: Input<CycleTime, "cycle_time">,
    has_ground_contact: Input<bool, "has_ground_contact">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub head_joints_command: MainOutput<HeadMotorCommand<f32>>,
}

impl HeadMotion {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            last_positions: Default::default(),
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let HeadMotorCommand {
            positions: raw_positions,
            stiffnesses,
        } = context
            .has_ground_contact
            .then(|| Self::joints_from_motion(&context))
            .unwrap_or_else(|| HeadMotorCommand {
                positions: Default::default(),
                stiffnesses: HeadJoints::fill(0.8),
            });

        let maximum_movement =
            *context.maximum_velocity * context.cycle_time.last_cycle_duration.as_secs_f32();

        let controlled_positions = HeadJoints {
            yaw: self.last_positions.yaw
                + (raw_positions.yaw - self.last_positions.yaw)
                    .clamp(-maximum_movement.yaw, maximum_movement.yaw),
            pitch: self.last_positions.pitch
                + (raw_positions.pitch - self.last_positions.pitch)
                    .clamp(-maximum_movement.pitch, maximum_movement.pitch),
        };

        let maximum_pitch = if controlled_positions.yaw.abs() >= *context.outer_yaw {
            *context.outer_maximum_pitch
        } else {
            let interpolation_factor =
                0.5 * (1.0 + (PI / *context.outer_yaw * controlled_positions.yaw).cos());
            *context.outer_maximum_pitch
                + interpolation_factor
                    * (*context.inner_maximum_pitch - *context.outer_maximum_pitch)
        };

        let clamped_pitch = controlled_positions.pitch.clamp(0.0, maximum_pitch);
        let clamped_positions = HeadJoints {
            pitch: clamped_pitch,
            yaw: controlled_positions.yaw,
        };

        self.last_positions = clamped_positions;
        Ok(MainOutputs {
            head_joints_command: HeadMotorCommand {
                positions: clamped_positions,
                stiffnesses,
            }
            .into(),
        })
    }

    pub fn joints_from_motion(context: &CycleContext) -> HeadMotorCommand<f32> {
        let stiffnesses = HeadJoints::fill(0.8);
        match context.motion_command.head_motion() {
            Some(HeadMotionCommand::Center) => HeadMotorCommand {
                positions: *context.center_head_position,
                stiffnesses,
            },
            Some(HeadMotionCommand::LookAround | HeadMotionCommand::SearchForLostBall) => {
                HeadMotorCommand {
                    positions: *context.look_around,
                    stiffnesses,
                }
            }
            Some(HeadMotionCommand::LookAt { .. })
            | Some(HeadMotionCommand::LookLeftAndRightOf { .. }) => HeadMotorCommand {
                positions: *context.look_at,
                stiffnesses,
            },
            Some(HeadMotionCommand::Unstiff) => HeadMotorCommand {
                positions: context.sensor_data.positions.head,
                stiffnesses: HeadJoints::fill(0.0),
            },
            Some(HeadMotionCommand::ZeroAngles) | None => HeadMotorCommand {
                positions: Default::default(),
                stiffnesses,
            },
        }
    }
}
