use nalgebra::{point, Isometry3, Point2};

use macros::{module, require_some};

use crate::control::modules::camera_matrix_provider::camera_to_head;
use crate::kinematics::{head_to_neck, neck_to_robot};
use crate::types::{
    CameraPosition, HeadJoints, HeadMotion, HeadMotionSafeExits, HeadMotionType, Joints, Motion,
    MotionCommand, MotionSelection, SensorData,
};

use crate::framework::configuration::{CameraMatrixParameters, LookAt as LookAtConfiguration};

pub struct LookAt {
    last_request: HeadJoints,
}

#[module(control)]
#[input(path = motion_selection, data_type = MotionSelection)]
#[input(path = motion_command, data_type = MotionCommand)]
#[input(path = sensor_data, data_type = SensorData)]
#[input(path = robot_to_ground, data_type = Isometry3<f32>)]
#[parameter(path = control.look_at, data_type = LookAtConfiguration)]
#[parameter(name = top_camera_matrix_parameters, path = vision_top.camera_matrix_parameters, data_type = CameraMatrixParameters)]
#[parameter(name = bottom_camera_matrix_parameters, path = vision_bottom.camera_matrix_parameters, data_type = CameraMatrixParameters)]
#[persistent_state(path = head_motion_safe_exits, data_type = HeadMotionSafeExits)]
#[main_output(name = look_at, data_type = HeadJoints)]
impl LookAt {}

impl LookAt {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {
            last_request: Default::default(),
        })
    }

    fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        let motion_selection = require_some!(context.motion_selection);
        let motion_command = require_some!(context.motion_command);
        let sensor_data = require_some!(context.sensor_data);
        let robot_to_ground = require_some!(context.robot_to_ground);
        let current_head_angles = sensor_data.positions.head;
        let configuration = context.look_at;

        context.head_motion_safe_exits[HeadMotionType::LookAt] = true;

        let default_output = Ok(MainOutputs {
            look_at: Some(current_head_angles),
        });

        if motion_selection.current_head_motion != HeadMotionType::LookAt {
            self.last_request = current_head_angles;
            return default_output;
        }

        let maximum_yaw_movement_next_cycle = configuration.maximum_yaw_velocity.to_radians()
            * sensor_data.cycle_info.last_cycle_duration.as_secs_f32();
        let maximum_pitch_movement_next_cycle = configuration.maximum_pitch_velocity.to_radians()
            * sensor_data.cycle_info.last_cycle_duration.as_secs_f32();

        let head_motion = match motion_command.motion {
            Motion::Kick { head, direction: _ } => head,
            Motion::SitDown { head } => head,
            Motion::Stand { head } => head,
            Motion::Walk {
                head,
                in_walk_kick: _,
                left_arm: _,
                right_arm: _,
                target_pose: _,
            } => head,
            _ => return default_output,
        };

        let target = match head_motion {
            HeadMotion::LookAt { target } => target,
            _ => return default_output,
        };

        let zero_angles = HeadJoints {
            yaw: 0.0,
            pitch: 0.0,
        };
        let head_to_robot = neck_to_robot(&zero_angles) * head_to_neck(&zero_angles);
        let head_to_ground = robot_to_ground * head_to_robot;
        let desired_angles = look_at(
            sensor_data.positions,
            target,
            configuration.bottom_focus_pitch_threshold,
            context.top_camera_matrix_parameters,
            context.bottom_camera_matrix_parameters,
            head_to_ground,
        );
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

        let interpolation_factor = 0.5 * ((request.yaw * 2.0).cos() + 1.0);
        let upper_pitch_limit = if request.yaw.abs()
            > configuration.yaw_threshold_for_pitch_limit.to_radians()
        {
            configuration.maximum_pitch_at_shoulder.to_radians()
        } else {
            (configuration.maximum_pitch_at_shoulder
                + (configuration.maximum_pitch_at_center - configuration.maximum_pitch_at_shoulder)
                    * interpolation_factor)
                .to_radians()
        };

        let clamped_request = HeadJoints {
            yaw: request
                .yaw
                .clamp(-configuration.maximum_yaw, configuration.maximum_yaw),
            pitch: request.pitch.clamp(f32::NEG_INFINITY, upper_pitch_limit),
        };
        self.last_request = clamped_request;

        context.head_motion_safe_exits[HeadMotionType::LookAt] = true;

        Ok(MainOutputs {
            look_at: Some(clamped_request),
        })
    }
}

fn look_at(
    joint_angles: Joints,
    target: Point2<f32>,
    bottom_focus_pitch_threshold: f32,
    top_camera_matrix_parameters: &CameraMatrixParameters,
    bottom_camera_matrix_parameters: &CameraMatrixParameters,
    head_to_ground: Isometry3<f32>,
) -> HeadJoints {
    let mut joint_angles_looking_forward = joint_angles;
    joint_angles_looking_forward.head = HeadJoints::default();

    let top_camera_to_head = camera_to_head(
        CameraPosition::Top,
        top_camera_matrix_parameters.extrinsic_rotations,
    );
    let top_camera_to_ground = head_to_ground * top_camera_to_head;
    let ground_to_top_camera = top_camera_to_ground.inverse();

    let bottom_camera_to_head = camera_to_head(
        CameraPosition::Bottom,
        bottom_camera_matrix_parameters.extrinsic_rotations,
    );
    let bottom_camera_to_ground = head_to_ground * bottom_camera_to_head;
    let ground_to_bottom_camera = bottom_camera_to_ground.inverse();

    let top_focus_angles = look_at_with_camera(target, ground_to_top_camera);
    let bottom_focus_angles = look_at_with_camera(target, ground_to_bottom_camera);

    let total_movement_top = (top_focus_angles.yaw - joint_angles.head.yaw).abs()
        + (top_focus_angles.pitch - joint_angles.head.pitch).abs();
    let total_movement_bottom = (bottom_focus_angles.yaw - joint_angles.head.yaw).abs()
        + (bottom_focus_angles.yaw - joint_angles.head.pitch).abs();

    let top_camera_too_high_with_bottom_focus =
        bottom_focus_angles.pitch < bottom_focus_pitch_threshold;

    if !top_camera_too_high_with_bottom_focus && total_movement_top > total_movement_bottom {
        bottom_focus_angles
    } else {
        top_focus_angles
    }
}

fn look_at_with_camera(target: Point2<f32>, ground_to_camera: Isometry3<f32>) -> HeadJoints {
    let target_in_camera = ground_to_camera * point![target.x, target.y, 0.0];

    let yaw = f32::atan2(target_in_camera.y, target_in_camera.x);

    let pitch = -f32::atan2(target_in_camera.z, target_in_camera.x);

    HeadJoints { yaw, pitch }
}
