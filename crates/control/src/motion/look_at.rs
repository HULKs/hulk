use std::{time::Duration, time::SystemTime};

use color_eyre::Result;
use kinematics::forward::{head_to_neck, neck_to_robot};
use projection::{camera_matrices::CameraMatrices, camera_matrix::CameraMatrix};
use serde::{Deserialize, Serialize};

use context_attribute::context;
use coordinate_systems::{Camera, Field, Ground, Head, Robot};
use framework::MainOutput;
use linear_algebra::{distance, point, vector, Isometry3, Point2};
use types::{
    camera_position::CameraPosition,
    cycle_time::CycleTime,
    joints::{head::HeadJoints, Joints},
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
    camera_matrices: Input<Option<CameraMatrices>, "camera_matrices?">,
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
    minimum_bottom_focus_pitch: Parameter<f32, "look_at.minimum_bottom_focus_pitch">,
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

        let camera_matrices = match context.camera_matrices {
            Some(camera_matrices) => camera_matrices,
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

        let (target, image_region_target, camera) = match *head_motion {
            HeadMotion::LookAt {
                target,
                image_region_target,
                camera,
            } => (target, image_region_target, camera),
            HeadMotion::LookAtReferee {
                image_region_target,
                camera,
            } => (expected_referee_position, image_region_target, camera),
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
                    None,
                )
            }
            _ => return default_output,
        };

        let zero_head_to_robot =
            neck_to_robot(&HeadJoints::default()) * head_to_neck(&HeadJoints::default());
        let robot_to_zero_head = zero_head_to_robot.inverse();
        let ground_to_zero_head = robot_to_zero_head * ground_to_robot;

        let request = match camera {
            Some(camera) => {
                let camera_matrix = match camera {
                    CameraPosition::Top => &camera_matrices.top,
                    CameraPosition::Bottom => &camera_matrices.bottom,
                };
                look_at_with_camera(
                    target,
                    camera_matrix.head_to_camera * ground_to_zero_head,
                    camera_matrix,
                    image_region_target,
                    *context.image_region_parameters,
                )
            }
            None => look_at(
                context.sensor_data.positions,
                ground_to_zero_head,
                camera_matrices,
                ImageRegion::default(),
                target,
                *context.minimum_bottom_focus_pitch,
                *context.image_region_parameters,
            ),
        };

        Ok(MainOutputs {
            look_at: request.into(),
        })
    }
}

fn look_at(
    joint_angles: Joints<f32>,
    ground_to_zero_head: Isometry3<Ground, Head>,
    camera_matrices: &CameraMatrices,
    image_region_target: ImageRegion,
    target: Point2<Ground>,
    minimum_bottom_focus_pitch: f32,
    image_region_parameters: ImageRegionParameters,
) -> HeadJoints<f32> {
    let head_to_top_camera = camera_matrices.top.head_to_camera;
    let head_to_bottom_camera = camera_matrices.bottom.head_to_camera;

    let top_focus_angles = look_at_with_camera(
        target,
        head_to_top_camera * ground_to_zero_head,
        &camera_matrices.top,
        image_region_target,
        image_region_parameters,
    );
    let bottom_focus_angles = look_at_with_camera(
        target,
        head_to_bottom_camera * ground_to_zero_head,
        &camera_matrices.bottom,
        image_region_target,
        image_region_parameters,
    );

    let pitch_movement_top = (top_focus_angles.pitch - joint_angles.head.pitch).abs();
    let pitch_movement_bottom = (bottom_focus_angles.pitch - joint_angles.head.pitch).abs();

    let force_top_focus = bottom_focus_angles.pitch < minimum_bottom_focus_pitch;

    if force_top_focus || pitch_movement_top < pitch_movement_bottom {
        top_focus_angles
    } else {
        bottom_focus_angles
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
