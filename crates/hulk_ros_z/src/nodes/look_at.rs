use std::{future::pending, sync::Arc, time::Duration};

use booster::MotorState;
use color_eyre::Result;
use coordinate_systems::{Ground, Robot};
use kinematics::joints::{Joints, head::HeadJoints};
use linear_algebra::Isometry3;
use projection::camera_matrix::CameraMatrix;
use ros_z::prelude::*;
use serde::{Deserialize, Serialize};
use types::{motion_command::MotionCommand, parameters::ImageRegionParameters};

use crate::IntoEyreResultExt;

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub glance_angle: f32,
    pub image_region_parameters: ImageRegionParameters,
    pub glance_direction_toggle_interval: Duration,
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("look_at").build().await.into_eyre()?;

    let _parameters = node
        .bind_parameter_as::<Parameters>("look_at")
        .into_eyre()?;
    let _camera_matrix_sub = node
        .subscriber::<CameraMatrix>("camera_matrix")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _ground_to_robot_sub = node
        .subscriber::<Isometry3<Ground, Robot>>("ground_to_robot")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _motion_command_sub = node
        .subscriber::<MotionCommand>("motion_command")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _serial_motor_states_sub = node
        .subscriber::<Joints<MotorState>>("serial_motor_states")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _look_at_pub = node
        .publisher::<HeadJoints<f32>>("look_at")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;

    pending::<()>().await;

    Ok(())
}
