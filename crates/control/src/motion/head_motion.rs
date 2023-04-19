use std::f32::consts::PI;

use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use types::{
    CycleTime, HeadJoints, HeadJointsCommand, HeadMotion as HeadMotionCommand, MotionCommand,
    SensorData,
};

#[derive(Default)]
pub struct HeadMotion {
    last_request: HeadJoints<f32>,
}

#[context]
pub struct CreationContext {
    pub center_head_position: Parameter<HeadJoints<f32>, "center_head_position">,
    pub inner_maximum_pitch: Parameter<f32, "head_motion.inner_maximum_pitch">,
    pub maximum_velocity: Parameter<HeadJoints<f32>, "head_motion.maximum_velocity">,
    pub outer_maximum_pitch: Parameter<f32, "head_motion.outer_maximum_pitch">,
    pub outer_yaw: Parameter<f32, "head_motion.outer_yaw">,
}

#[context]
pub struct CycleContext {
    pub center_head_position: Parameter<HeadJoints<f32>, "center_head_position">,
    pub inner_maximum_pitch: Parameter<f32, "head_motion.inner_maximum_pitch">,
    pub maximum_velocity: Parameter<HeadJoints<f32>, "head_motion.maximum_velocity">,
    pub outer_maximum_pitch: Parameter<f32, "head_motion.outer_maximum_pitch">,
    pub outer_yaw: Parameter<f32, "head_motion.outer_yaw">,

    pub look_around: Input<HeadJoints<f32>, "look_around">,
    pub look_at: Input<HeadJoints<f32>, "look_at">,
    pub motion_command: Input<MotionCommand, "motion_command">,
    pub sensor_data: Input<SensorData, "sensor_data">,
    pub cycle_time: Input<CycleTime, "cycle_time">,
    pub has_ground_contact: Input<bool, "has_ground_contact">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub head_joints_command: MainOutput<HeadJointsCommand<f32>>,
}

impl HeadMotion {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            last_request: Default::default(),
        })
    }

    pub fn if_ground_contact(context: CycleContext, head_joints: HeadJoints) -> HeadJoints {
        if *context.has_ground_contact {
            head_joints
        } else {
            Default::default()
        }
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let current_head_angles = context.sensor_data.positions.head;
        let raw_request = match context.motion_command.head_motion() {
            Some(HeadMotionCommand::Center) => *context.center_head_position,
            Some(HeadMotionCommand::LookAround) | Some(HeadMotionCommand::SearchForLostBall) => {
                if *context.has_ground_contact {
                    *context.look_around
                } else {
                    Default::default()
                }
            }
            Some(HeadMotionCommand::LookAt { .. }) => {
                if *context.has_ground_contact {
                    *context.look_at
                } else {
                    Default::default()
                }
            }
            Some(HeadMotionCommand::LookLeftAndRightOf { .. }) => *context.look_at,
            Some(HeadMotionCommand::Unstiff) => current_head_angles,
            Some(HeadMotionCommand::ZeroAngles) => Default::default(),
            None => Default::default(),
        };
        let maximum_movement =
            *context.maximum_velocity * context.cycle_time.last_cycle_duration.as_secs_f32();

        let controlled_request = HeadJoints {
            yaw: self.last_request.yaw
                + (raw_request.yaw - self.last_request.yaw)
                    .clamp(-maximum_movement.yaw, maximum_movement.yaw),
            pitch: self.last_request.pitch
                + (raw_request.pitch - self.last_request.pitch)
                    .clamp(-maximum_movement.pitch, maximum_movement.pitch),
        };

        let pitch_max = if controlled_request.yaw.abs() >= *context.outer_yaw {
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
            head_joints_command: HeadJointsCommand {
                positions: clamped_request,
                stiffnesses: HeadJoints::fill(0.8),
            }
            .into(),
        })
    }
}
