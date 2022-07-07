use nalgebra::{point, Isometry3, Point2};

use module_derive::{module, require_some};
use types::{
    CameraMatrices, HeadJoints, HeadMotion, Joints, MotionCommand, RobotKinematics, SensorData,
};

use crate::{kinematics::head_to_neck, kinematics::neck_to_robot};

pub struct LookAt {}

#[module(control)]
#[input(path = robot_kinematics, data_type = RobotKinematics)]
#[input(path = motion_command, data_type = MotionCommand)]
#[input(path = sensor_data, data_type = SensorData)]
#[input(path = camera_matrices, data_type = CameraMatrices)]
#[input(path = ground_to_robot, data_type = Isometry3<f32>)]
#[parameter(path = control.look_at.minimum_bottom_focus_pitch, data_type = f32)]
#[main_output(name = look_at, data_type = HeadJoints)]
impl LookAt {}

impl LookAt {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {})
    }

    fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        let motion_command = require_some!(context.motion_command);
        let sensor_data = require_some!(context.sensor_data);
        let current_head_angles = sensor_data.positions.head;

        let default_output = Ok(MainOutputs {
            look_at: Some(current_head_angles),
        });

        let camera_matrices = match context.camera_matrices {
            Some(camera_matrices) => camera_matrices,
            None => return default_output,
        };

        let ground_to_robot = match context.ground_to_robot {
            Some(ground_to_robot) => ground_to_robot,
            None => return default_output,
        };

        let head_motion = match motion_command {
            MotionCommand::Kick { head, direction: _ } => head,
            MotionCommand::SitDown { head } => head,
            MotionCommand::Stand { head } => head,
            MotionCommand::Walk { head, .. } => head,
            _ => return default_output,
        };

        let target = match head_motion {
            HeadMotion::LookAt { target } => target,
            _ => return default_output,
        };

        let zero_head_to_robot =
            neck_to_robot(&HeadJoints::default()) * head_to_neck(&HeadJoints::default());
        let ground_to_zero_head = zero_head_to_robot.inverse() * ground_to_robot;

        let request = look_at(
            sensor_data.positions,
            ground_to_zero_head,
            camera_matrices.top.camera_to_head.inverse(),
            camera_matrices.bottom.camera_to_head.inverse(),
            *target,
            *context.minimum_bottom_focus_pitch,
        );

        Ok(MainOutputs {
            look_at: Some(request),
        })
    }
}

fn look_at(
    joint_angles: Joints,
    ground_to_zero_head: Isometry3<f32>,
    head_to_top_camera: Isometry3<f32>,
    head_to_bottom_camera: Isometry3<f32>,
    target: Point2<f32>,
    minimum_bottom_focus_pitch: f32,
) -> HeadJoints {
    let top_focus_angles = look_at_with_camera(target, head_to_top_camera * ground_to_zero_head);
    let bottom_focus_angles =
        look_at_with_camera(target, head_to_bottom_camera * ground_to_zero_head);

    let pitch_movement_top = (top_focus_angles.pitch - joint_angles.head.pitch).abs();
    let pitch_movement_bottom = (bottom_focus_angles.pitch - joint_angles.head.pitch).abs();

    let force_top_focus = bottom_focus_angles.pitch < minimum_bottom_focus_pitch;

    if force_top_focus || pitch_movement_top < pitch_movement_bottom {
        top_focus_angles
    } else {
        bottom_focus_angles
    }
}

fn look_at_with_camera(target: Point2<f32>, ground_to_camera: Isometry3<f32>) -> HeadJoints {
    let target_in_camera = ground_to_camera * point![target.x, target.y, 0.0];
    let yaw = f32::atan2(target_in_camera.y, target_in_camera.x);
    let pitch = -f32::atan2(target_in_camera.z, target_in_camera.x);
    HeadJoints { yaw, pitch }
}
