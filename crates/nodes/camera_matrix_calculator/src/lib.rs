use std::{f32::consts::FRAC_PI_2, sync::Arc};

use color_eyre::Result;

use coordinate_systems::{Camera, Ground, Head, Robot};
use kinematics::{robot_dimensions::RobotDimensions, robot_kinematics::RobotKinematics};
use linear_algebra::{IntoTransform, Isometry3, Vector3, vector};
use projection::camera_matrix::CameraMatrix;
use ros_z::prelude::*;
use ros2::sensor_msgs::camera_info::CameraInfo;
use types::parameters::CameraMatrixParameters;

pub const ACTUAL_IMAGE_HEIGHT: f32 = 448.0;
pub const ACTUAL_IMAGE_WIDTH: f32 = 544.0;

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("camera_matrix_calculator").build().await?;

    let parameters =
        node.bind_parameter_as::<CameraMatrixParameters>("camera_matrix_calculator")?;
    let robot_kinematics_sub = node
        .subscriber::<RobotKinematics>("robot_kinematics")?
        .build()
        .await?;
    let robot_to_ground_cache = node
        .create_cache::<Option<Isometry3<Robot, Ground>>>("robot_to_ground", 10)?
        .build()
        .await?;
    let camera_info_cache = node
        .create_cache::<CameraInfo>("inputs/camera_info", 1)?
        .build()
        .await?;

    let camera_matrix_pub = node
        .publisher::<CameraMatrix>("camera_matrix")?
        .build()
        .await?;

    loop {
        let parameters = parameters.snapshot().typed().clone();

        let robot_kinematics = robot_kinematics_sub.recv_with_metadata().await?;

        let time_stamp = robot_kinematics.source_time;

        let maybe_robot_to_ground = robot_to_ground_cache.get_nearest(time_stamp);
        let maybe_camera_info = camera_info_cache.get_nearest(time_stamp);

        let (Some(maybe_robot_to_ground), Some(camera_info)) =
            (maybe_robot_to_ground, maybe_camera_info)
        else {
            continue;
        };
        let Some(robot_to_ground) = *maybe_robot_to_ground else {
            continue;
        };

        let camera_matrix = compute_camera_matrix(
            &parameters,
            &robot_kinematics,
            &robot_to_ground,
            &camera_info,
        );

        camera_matrix_pub.publish(&camera_matrix).await?;
    }
}

fn compute_camera_matrix(
    parameters: &CameraMatrixParameters,
    robot_kinematics: &RobotKinematics,
    robot_to_ground: &Isometry3<Robot, Ground>,
    camera_info: &CameraInfo,
) -> CameraMatrix {
    // This is a hack, since the camera info currently received by the X5Receiver is wrong.
    let image_size = vector!(ACTUAL_IMAGE_WIDTH, ACTUAL_IMAGE_HEIGHT);
    let head_to_camera = head_to_camera(
        parameters.camera_to_head_pitch.to_radians(),
        RobotDimensions::HEAD_TO_CAMERA,
    );

    CameraMatrix::from_camera_info(
        camera_info,
        image_size,
        robot_to_ground.inverse(),
        robot_kinematics.head.head_to_robot.inverse(),
        head_to_camera,
    )
}

fn head_to_camera(camera_pitch: f32, head_to_camera: Vector3<Head>) -> Isometry3<Head, Camera> {
    (nalgebra::Isometry3::rotation(nalgebra::Vector3::x() * -camera_pitch)
        * nalgebra::Isometry3::rotation(nalgebra::Vector3::y() * -FRAC_PI_2)
        * nalgebra::Isometry3::rotation(nalgebra::Vector3::x() * FRAC_PI_2)
        * nalgebra::Isometry3::from(-head_to_camera.inner))
    .framed_transform()
}
