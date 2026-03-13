use booster::{JointsMotorState, MotorState};
use color_eyre::Result;
use context_attribute::context;
use filtering::low_pass_filter::LowPassFilter;
use framework::MainOutput;
use kinematics::joints::{Joints, head::HeadJoints};
use projection::camera_matrix::CameraMatrix;
use serde::{Deserialize, Serialize};
use types::{
    cycle_time::CycleTime,
    motion_command::{HeadMotion as HeadMotionCommand, ImageRegion, MotionCommand},
    parameters::HeadMotionParameters,
};

#[derive(Default, Deserialize, Serialize)]
pub struct HeadMotion {
    last_positions: HeadJoints<f32>,
    lowpass_filter: LowPassFilter<HeadJoints<f32>>,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    parameters: Parameter<HeadMotionParameters, "head_motion">,
    look_around_target_joints: Input<HeadJoints<f32>, "look_around_target_joints">,
    look_at: Input<HeadJoints<f32>, "look_at">,
    motor_states: Input<Joints<MotorState>, "serial_motor_states">,
    cycle_time: Input<CycleTime, "cycle_time">,
    motion_command: Input<MotionCommand, "WorldState", "motion_command">,
    camera_matrix: Input<Option<CameraMatrix>, "WorldState", "camera_matrix?">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub head_joints_command: MainOutput<HeadJoints<f32>>,
}

impl HeadMotion {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            last_positions: Default::default(),
            lowpass_filter: LowPassFilter::with_smoothing_factor(Default::default(), 0.075),
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        if let Some(injected_head_joints) = context.parameters.injected_head_joints {
            self.lowpass_filter.update(injected_head_joints);

            return Ok(MainOutputs {
                head_joints_command: self.lowpass_filter.state().into(),
            });
        }
        let raw_positions = Self::joints_from_motion(&context);
        let maximum_movement = context.parameters.maximum_velocity
            * context.cycle_time.last_cycle_duration.as_secs_f32();

        let max_pitch = if let Some(camera_matrix) = context.camera_matrix
            && let Some(horizon) = camera_matrix.horizon
        {
            let cy = camera_matrix.intrinsics.optical_center.y();
            let horizon_y = horizon.vanishing_point.y();
            let max_pitch = f32::atan2(horizon_y - cy, camera_matrix.intrinsics.focals.y);
            maximum_movement
                .pitch
                .min(max_pitch)
                .max(-maximum_movement.pitch + 10f32.to_radians())
        } else {
            maximum_movement.pitch
        };

        let controlled_positions = HeadJoints {
            yaw: self.last_positions.yaw
                + (raw_positions.yaw - self.last_positions.yaw)
                    .clamp(-maximum_movement.yaw, maximum_movement.yaw),
            pitch: self.last_positions.pitch
                + (raw_positions.pitch - self.last_positions.pitch)
                    .clamp(-maximum_movement.pitch, max_pitch),
        };

        let clamped_positions = HeadJoints {
            pitch: controlled_positions.pitch.clamp(
                context.parameters.minimum_pitch,
                context.parameters.maximum_pitch,
            ),
            yaw: controlled_positions.yaw.clamp(
                context.parameters.minimum_yaw,
                context.parameters.maximum_yaw,
            ),
        };

        self.last_positions = clamped_positions;
        Ok(MainOutputs {
            head_joints_command: clamped_positions.into(),
        })
    }

    pub fn joints_from_motion(context: &CycleContext) -> HeadJoints<f32> {
        match context.motion_command.head_motion() {
            Some(HeadMotionCommand::Center {
                image_region_target: ImageRegion::Top,
            }) => HeadJoints {
                yaw: 0.0,
                pitch: 0.4,
            },
            Some(HeadMotionCommand::Center { .. }) => HeadJoints {
                yaw: 0.0,
                pitch: 0.4,
            },
            Some(HeadMotionCommand::LookAt { .. })
            | Some(HeadMotionCommand::LookAtReferee { .. })
            | Some(HeadMotionCommand::LookLeftAndRightOf { .. }) => *context.look_at,
            Some(HeadMotionCommand::Unstiff) => context.motor_states.positions().head,
            Some(HeadMotionCommand::Animation { stiff: false }) => {
                context.motor_states.positions().head
            }
            Some(HeadMotionCommand::Animation { stiff: true }) => {
                context.motor_states.positions().head
            }
            Some(HeadMotionCommand::LookAround) | Some(HeadMotionCommand::SearchForLostBall) => {
                *context.look_around_target_joints
            }
            Some(_) | None => Default::default(),
        }
    }
}
