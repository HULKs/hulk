use std::{time::Duration, time::SystemTime};

use color_eyre::Result;
use projection::camera_matrices::CameraMatrices;
use serde::{Deserialize, Serialize};

use context_attribute::context;
use coordinate_systems::{Camera, Ground, Head, Pixel, Robot};
use framework::MainOutput;
use kinematics::forward::{head_to_neck, neck_to_robot};
use linear_algebra::{distance, point, vector, Isometry3, Point2};
use types::{
    camera_position::CameraPosition,
    cycle_time::CycleTime,
    joints::{head::HeadJoints, Joints},
    motion_command::{GlanceDirection, HeadMotion, MotionCommand},
    sensor_data::SensorData,
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

    glance_angle: Parameter<f32, "look_at.glance_angle">,
    glance_direction_toggle_interval:
        Parameter<Duration, "look_at.glance_direction_toggle_interval">,
    offset_in_image: Parameter<Point2<Pixel>, "look_at.glance_center_offset_in_image">,
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
        let current_head_angles = context.sensor_data.positions.head;
        let default_output = Ok(MainOutputs {
            look_at: current_head_angles.into(),
        });

        let camera_matrices = match context.camera_matrices {
            Some(camera_matrices) => camera_matrices,
            None => return default_output,
        };

        let ground_to_robot = match context.ground_to_robot {
            Some(ground_to_robot) => *ground_to_robot,
            None => return default_output,
        };

        let head_motion = match context.motion_command {
            MotionCommand::Initial { head } => head,
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

        let (target, pixel_target, camera) = match head_motion {
            HeadMotion::LookAt {
                target,
                pixel_target,
                camera,
            } => (*target, *pixel_target, *camera),
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
                    Point2::origin(),
                    None,
                )
            }
            _ => return default_output,
        };

        dbg!("LookAt");

        let zero_head_to_robot =
            neck_to_robot(&HeadJoints::default()) * head_to_neck(&HeadJoints::default());
        let robot_to_zero_head = zero_head_to_robot.inverse();
        let ground_to_zero_head = robot_to_zero_head * ground_to_robot;

        let request = match camera {
            Some(camera) => {
                let (head_to_camera, focal_length, optical_center) = match camera {
                    CameraPosition::Top => (
                        camera_matrices.top.head_to_camera,
                        camera_matrices.top.focal_length,
                        camera_matrices.top.optical_center,
                    ),
                    CameraPosition::Bottom => (
                        camera_matrices.bottom.head_to_camera,
                        camera_matrices.bottom.focal_length,
                        camera_matrices.bottom.optical_center,
                    ),
                };
                dbg!("Camera position");
                dbg!(target);
                dbg!(pixel_target);
                look_at_with_camera(
                    target,
                    head_to_camera * ground_to_zero_head,
                    pixel_target,
                    optical_center,
                    focal_length.into(),
                )
            }
            None => look_at(
                context.sensor_data.positions,
                ground_to_zero_head,
                camera_matrices,
                *context.offset_in_image,
                target,
                *context.minimum_bottom_focus_pitch,
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
    pixel_target: Point2<Pixel>,
    target: Point2<Ground>,
    minimum_bottom_focus_pitch: f32,
) -> HeadJoints<f32> {
    let head_to_top_camera = camera_matrices.top.head_to_camera;
    let head_to_bottom_camera = camera_matrices.bottom.head_to_camera;
    let focal_length_top = camera_matrices.top.focal_length;
    let focal_length_bottom = camera_matrices.bottom.focal_length;
    let optical_center_top = camera_matrices.top.optical_center;
    let optical_center_bottom = camera_matrices.bottom.optical_center;

    let top_focus_angles = look_at_with_camera(
        target,
        head_to_top_camera * ground_to_zero_head,
        pixel_target,
        optical_center_top,
        focal_length_top.into(),
    );
    let bottom_focus_angles = look_at_with_camera(
        target,
        head_to_bottom_camera * ground_to_zero_head,
        pixel_target,
        optical_center_bottom,
        focal_length_bottom.into(),
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
    ground_to_camera: Isometry3<Ground, Camera>,
    pixel_target: Point2<Pixel>,
    optical_center: Point2<Pixel>,
    focal_length: nalgebra::Vector2<f32>,
) -> HeadJoints<f32> {
    let target_in_camera = ground_to_camera * point![target.x(), target.y(), 0.0];

    let offset_to_center = pixel_target - optical_center.coords;
    let yaw_offset = f32::atan2(offset_to_center.x, focal_length.x);
    let pitch_offset = f32::atan2(offset_to_center.y, focal_length.y);

    let yaw = f32::atan2(-target_in_camera.x(), target_in_camera.z()) + yaw_offset;
    let pitch = -f32::atan2(-target_in_camera.y(), target_in_camera.z()) - pitch_offset;

    HeadJoints { yaw, pitch }
}
