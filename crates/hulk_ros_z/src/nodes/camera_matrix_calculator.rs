use std::{future::pending, sync::Arc};

use color_eyre::Result;
use coordinate_systems::{Camera, Ground, Robot};
use kinematics::robot_kinematics::RobotKinematics;
use linear_algebra::{Isometry3, Vector3};
use projection::camera_matrix::CameraMatrix;
use ros_z::prelude::*;
use ros2::sensor_msgs::camera_info::CameraInfo;
use serde::{Deserialize, Serialize};
use types::parameters::CameraMatrixParameters;

use crate::IntoEyreResultExt;

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub camera_matrix_parameters: CameraMatrixParameters,
    pub correction_in_robot: Vector3<Robot>,
    pub correction_in_camera: Vector3<Camera>,
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx
        .create_node("camera_matrix_calculator")
        .build()
        .await
        .into_eyre()?;

    let _parameters = node
        .bind_parameter_as::<Parameters>("camera_matrix_calculator")
        .into_eyre()?;
    let _robot_kinematics_sub = node
        .subscriber::<RobotKinematics>("robot_kinematics")
        .build()
        .await
        .into_eyre()?;
    let _robot_to_ground_sub = node
        .subscriber::<Isometry3<Robot, Ground>>("robot_to_ground")
        .build()
        .await
        .into_eyre()?;
    let _image_left_raw_camera_info_sub = node
        .subscriber::<CameraInfo>("image_left_raw_camera_info")
        .build()
        .await
        .into_eyre()?;
    let _uncalibrated_camera_matrix_pub = node
        .publisher::<CameraMatrix>("uncalibrated_camera_matrix")
        .build()
        .await
        .into_eyre()?;
    let _camera_matrix_pub = node
        .publisher::<CameraMatrix>("camera_matrix")
        .build()
        .await
        .into_eyre()?;

    pending::<()>().await;

    Ok(())
}
