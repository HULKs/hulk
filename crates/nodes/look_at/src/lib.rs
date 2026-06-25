use std::{boxed::Box, future::Future, pin::Pin, sync::Arc, time::Duration};

use color_eyre::Result;
use serde::{Deserialize, Serialize};

use booster::MotorState;
use coordinate_systems::{Camera, Ground, Robot};
use kinematics::{
    forward::{head_to_neck, neck_to_robot},
    joints::{Joints, head::HeadJoints},
};
use linear_algebra::{Isometry3, Point2, distance, point, vector};
use projection::camera_matrix::CameraMatrix;
use ros_z::{prelude::*, time::Time};
use types::{
    motion_command::{GlanceDirection, HeadMotion, ImageRegion, MotionCommand},
    parameters::ImageRegionParameters,
    time_wrapper::TimeWrapper,
};

const MOTION_COMMAND_TOPIC: &str = "behavior/motion_command";

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub glance_angle: f32,
    pub image_region_parameters: ImageRegionParameters,
    pub glance_direction_toggle_interval: Duration,
}

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("look_at").build().await?;

    let parameters = node.bind_parameter_as::<Parameters>("look_at")?;
    let camera_matrix_cache = node
        .create_cache::<TimeWrapper<CameraMatrix>>("camera_matrix", 1)?
        .with_stamp(|wrapper: &TimeWrapper<CameraMatrix>| wrapper.time)
        .build()
        .await?;
    let ground_to_robot_cache = node
        .create_cache::<TimeWrapper<Option<Isometry3<Ground, Robot>>>>("ground_to_robot", 1)?
        .with_stamp(|wrapper: &TimeWrapper<Option<Isometry3<Ground, Robot>>>| wrapper.time)
        .build()
        .await?;
    let motion_command_cache = node
        .create_cache::<MotionCommand>(MOTION_COMMAND_TOPIC, 1)?
        .build()
        .await?;
    let serial_motor_states_cache = node
        .create_cache::<Joints<MotorState>>("inputs/serial_motor_states", 1)?
        .build()
        .await?;
    let look_at_pub = node
        .publisher::<HeadJoints<f32>>("look_at")?
        .build()
        .await?;

    let mut state = LookAtState::new();
    let mut tick = node.create_timer(Duration::from_millis(10));

    loop {
        tick.tick().await;
        let now = node.clock().now();
        let Some(serial_motor_states) = serial_motor_states_cache.get_latest() else {
            continue;
        };
        let camera_matrix = camera_matrix_cache.get_latest();
        let ground_to_robot = ground_to_robot_cache.get_latest();
        let motion_command = motion_command_cache.get_latest();

        let parameters_snapshot = parameters.snapshot();
        let parameters = parameters_snapshot.typed();
        let look_at = state.compute_head_joints(
            now,
            camera_matrix.as_deref().map(|wrapper| &wrapper.inner),
            ground_to_robot.as_deref().and_then(|wrapper| wrapper.inner),
            motion_command.as_deref(),
            &serial_motor_states,
            parameters,
        );
        look_at_pub.publish(&look_at).await?;
    }
}

#[derive(Debug)]
struct LookAtState {
    current_glance_direction: GlanceDirection,
    last_glance_direction_toggle: Option<Time>,
}

impl LookAtState {
    fn new() -> Self {
        Self {
            current_glance_direction: Default::default(),
            last_glance_direction_toggle: None,
        }
    }

    fn compute_head_joints(
        &mut self,
        now: Time,
        camera_matrix: Option<&CameraMatrix>,
        ground_to_robot: Option<Isometry3<Ground, Robot>>,
        motion_command: Option<&MotionCommand>,
        serial_motor_states: &Joints<MotorState>,
        parameters: &Parameters,
    ) -> HeadJoints<f32> {
        let measured_head_angles = measured_head_angles(serial_motor_states);

        let Some(camera_matrix) = camera_matrix else {
            return measured_head_angles;
        };
        let Some(ground_to_robot) = ground_to_robot else {
            return measured_head_angles;
        };
        let Some(head_motion) = motion_command.and_then(MotionCommand::head_motion) else {
            return measured_head_angles;
        };

        self.update_glance_direction(now, parameters.glance_direction_toggle_interval);

        let (target, image_region_target, with_camera) = match head_motion {
            HeadMotion::LookAt {
                target,
                image_region_target,
            } => (target, image_region_target, true),
            HeadMotion::LookLeftAndRightOf { target } => {
                let left_right_shift = vector![
                    0.0,
                    f32::tan(parameters.glance_angle) * distance(target, Point2::origin())
                ];
                (
                    match self.current_glance_direction {
                        GlanceDirection::LeftOfTarget => target + left_right_shift,
                        GlanceDirection::RightOfTarget => target - left_right_shift,
                    },
                    ImageRegion::default(),
                    false,
                )
            }
            _ => return measured_head_angles,
        };

        let zero_head_to_robot =
            neck_to_robot(&HeadJoints::default()) * head_to_neck(&HeadJoints::default());
        let robot_to_zero_head = zero_head_to_robot.inverse();
        let ground_to_zero_head = robot_to_zero_head * ground_to_robot;
        let image_region_target = if with_camera {
            image_region_target
        } else {
            ImageRegion::default()
        };

        look_at_with_camera(
            target,
            camera_matrix.head_to_camera * ground_to_zero_head,
            camera_matrix,
            image_region_target,
            parameters.image_region_parameters,
        )
    }

    fn update_glance_direction(&mut self, now: Time, toggle_interval: Duration) {
        let should_toggle = match self.last_glance_direction_toggle {
            Some(last_toggle) => now.duration_since(last_toggle) > toggle_interval,
            None => true,
        };

        if !should_toggle {
            return;
        }

        self.current_glance_direction = match self.current_glance_direction {
            GlanceDirection::LeftOfTarget => GlanceDirection::RightOfTarget,
            GlanceDirection::RightOfTarget => GlanceDirection::LeftOfTarget,
        };
        self.last_glance_direction_toggle = Some(now);
    }
}

fn look_at_with_camera(
    target: Point2<Ground>,
    ground_to_zero_camera: Isometry3<Ground, Camera>,
    camera_matrix: &CameraMatrix,
    image_region_target: ImageRegion,
    image_region_parameters: ImageRegionParameters,
) -> HeadJoints<f32> {
    let pixel_target = match image_region_target {
        ImageRegion::Center => image_region_parameters.center,
        ImageRegion::Bottom => image_region_parameters.bottom,
        ImageRegion::Top => image_region_parameters.top,
    };

    let pixel_target = point![
        pixel_target.x() * camera_matrix.image_size.x(),
        pixel_target.y() * camera_matrix.image_size.y()
    ];

    let target_in_camera = ground_to_zero_camera * point![target.x(), target.y(), 0.0];

    let offset_to_center = pixel_target - camera_matrix.intrinsics.optical_center.coords();
    let yaw_offset = f32::atan2(offset_to_center.x(), camera_matrix.intrinsics.focals.x);
    let pitch_offset = f32::atan2(offset_to_center.y(), camera_matrix.intrinsics.focals.y);

    let yaw = f32::atan2(-target_in_camera.x(), target_in_camera.z()) + yaw_offset;
    let pitch = -f32::atan2(-target_in_camera.y(), target_in_camera.z()) - pitch_offset;

    HeadJoints { yaw, pitch }
}

fn measured_head_angles(serial_motor_states: &Joints<MotorState>) -> HeadJoints<f32> {
    HeadJoints {
        yaw: serial_motor_states.head.yaw.position,
        pitch: serial_motor_states.head.pitch.position,
    }
}

#[cfg(test)]
mod tests {
    use coordinate_systems::{Camera, Ground, Head, Robot};
    use kinematics::joints::head::HeadJoints;
    use linear_algebra::{Isometry3, nalgebra, point};
    use projection::camera_matrix::CameraMatrix;
    use types::{motion_command::ImageRegion, parameters::ImageRegionParameters};

    use super::*;

    #[test]
    fn measured_head_angles_read_head_motor_positions_directly() {
        let mut motor_states = Joints::fill(MotorState::default());
        motor_states.head.yaw.position = 0.3;
        motor_states.head.pitch.position = -0.2;

        assert_eq!(
            measured_head_angles(&motor_states),
            HeadJoints {
                yaw: 0.3,
                pitch: -0.2,
            }
        );
    }

    #[test]
    fn centered_target_requires_zero_angles_with_centered_camera_region() {
        let camera_matrix = CameraMatrix::from_normalized_focal_and_center(
            nalgebra::vector![1.0, 1.0],
            nalgebra::point![0.5, 0.5],
            linear_algebra::vector![640.0, 480.0],
            Isometry3::<Ground, Robot>::identity(),
            Isometry3::<Robot, Head>::identity(),
            Isometry3::<Head, Camera>::from_translation(0.0, 0.0, 1.0),
        );
        let image_region_parameters = ImageRegionParameters {
            bottom: point![0.5, 0.5],
            center: point![0.5, 0.5],
            top: point![0.5, 0.5],
        };

        let HeadJoints { yaw, pitch } = look_at_with_camera(
            point![0.0, 0.0],
            Isometry3::<Ground, Camera>::identity(),
            &camera_matrix,
            ImageRegion::Center,
            image_region_parameters,
        );

        assert!(yaw.abs() < f32::EPSILON);
        assert!(pitch.abs() < f32::EPSILON);
    }
}
