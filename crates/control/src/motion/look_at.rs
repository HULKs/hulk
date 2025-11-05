use std::{time::Duration, time::SystemTime};

use color_eyre::Result;
use kinematics::forward::{head_to_neck, neck_to_robot};
use projection::camera_matrix::CameraMatrix;
use serde::{Deserialize, Serialize};

use context_attribute::context;
use coordinate_systems::{Camera, Field, Ground, Head, Robot};
use framework::MainOutput;
use linear_algebra::{distance, point, vector, Isometry3, Point2};
use types::{
    cycle_time::CycleTime,
    joints::head::HeadJoints,
    motion_command::{GlanceDirection, HeadMotion, ImageRegion, MotionCommand},
    parameters::ImageRegionParameters,
    sensor_data::SensorData,
    world_state::WorldState,
};

#[derive(Deserialize, Serialize)]
pub struct LookAt {
    current_glance_direction: GlanceDirection,
    last_glance_direction_toggle: Option<SystemTime>,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    camera_matrix: Input<Option<CameraMatrix>, "camera_matrix?">,
    cycle_time: Input<CycleTime, "cycle_time">,
    ground_to_robot: Input<Option<Isometry3<Ground, Robot>>, "ground_to_robot?">,
    motion_command: Input<MotionCommand, "motion_command">,
    sensor_data: Input<SensorData, "sensor_data">,
    expected_referee_position: Input<Option<Point2<Field>>, "expected_referee_position?">,
    world_state: Input<WorldState, "world_state">,

    glance_angle: Parameter<f32, "look_at.glance_angle">,
    image_region_parameters: Parameter<ImageRegionParameters, "look_at.image_regions">,
    glance_direction_toggle_interval:
        Parameter<Duration, "look_at.glance_direction_toggle_interval">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub look_at: MainOutput<HeadJoints<f32>>,
}

impl LookAt {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            current_glance_direction: Default::default(),
            last_glance_direction_toggle: None,
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let cycle_start_time = context.cycle_time.start_time;
        let measured_head_angles = context.sensor_data.positions.head;
        let default_output = Ok(MainOutputs {
            look_at: measured_head_angles.into(),
        });

        let camera_matrix = match context.camera_matrix {
            Some(camera_matrix) => camera_matrix,
            None => return default_output,
        };

        let ground_to_robot = match context.ground_to_robot {
            Some(ground_to_robot) => *ground_to_robot,
            None => return default_output,
        };

        let ground_to_field = match context.world_state.robot.ground_to_field {
            Some(ground_to_robot) => ground_to_robot,
            None => return default_output,
        };

        let head_motion = match context.motion_command {
            MotionCommand::Initial { head, .. } => head,
            MotionCommand::SitDown { head } => head,
            MotionCommand::Stand { head, .. } => head,
            MotionCommand::Walk { head, .. } => head,
            MotionCommand::WalkWithVelocity { head, .. } => head,
            _ => return default_output,
        };

        if self.last_glance_direction_toggle.is_none()
            || cycle_start_time.duration_since(self.last_glance_direction_toggle.unwrap())?
                > *context.glance_direction_toggle_interval
        {
            self.current_glance_direction = match self.current_glance_direction {
                GlanceDirection::LeftOfTarget => GlanceDirection::RightOfTarget,
                GlanceDirection::RightOfTarget => GlanceDirection::LeftOfTarget,
            };
            self.last_glance_direction_toggle = Some(cycle_start_time);
        }

        let expected_referee_position = ground_to_field.inverse()
            * context
                .expected_referee_position
                .unwrap_or(&point!(0.0, 0.0));

        let (target, image_region_target, with_camera) = match *head_motion {
            HeadMotion::LookAt {
                target,
                image_region_target,
            } => (target, image_region_target, true),
            HeadMotion::LookAtReferee {
                image_region_target,
            } => (expected_referee_position, image_region_target, true),
            HeadMotion::LookLeftAndRightOf { target } => {
                let left_right_shift = vector![
                    0.0,
                    f32::tan(*context.glance_angle) * distance(target, Point2::origin())
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
            _ => return default_output,
        };

        let zero_head_to_robot =
            neck_to_robot(&HeadJoints::default()) * head_to_neck(&HeadJoints::default());
        let robot_to_zero_head = zero_head_to_robot.inverse();
        let ground_to_zero_head = robot_to_zero_head * ground_to_robot;

        let request = match with_camera {
            true => look_at_with_camera(
                target,
                camera_matrix.head_to_camera * ground_to_zero_head,
                camera_matrix,
                image_region_target,
                *context.image_region_parameters,
            ),
            false => look_at(
                ground_to_zero_head,
                camera_matrix,
                ImageRegion::default(),
                target,
                *context.image_region_parameters,
            ),
        };

        Ok(MainOutputs {
            look_at: request.into(),
        })
    }
}

fn look_at(
    ground_to_zero_head: Isometry3<Ground, Head>,
    camera_matrix: &CameraMatrix,
    image_region_target: ImageRegion,
    target: Point2<Ground>,
    image_region_parameters: ImageRegionParameters,
) -> HeadJoints<f32> {
    let head_to_camera = camera_matrix.head_to_camera;

    look_at_with_camera(
        target,
        head_to_camera * ground_to_zero_head,
        &camera_matrix,
        image_region_target,
        image_region_parameters,
    )
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
