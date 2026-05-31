use std::{boxed::Box, future::Future, pin::Pin};
use std::{future::pending, sync::Arc, time::Duration};

use color_eyre::Result;
use serde::{Deserialize, Serialize};

use booster::MotorState;
use coordinate_systems::{Ground, Robot};
use kinematics::joints::{Joints, head::HeadJoints};
use linear_algebra::Isometry3;
use projection::camera_matrix::CameraMatrix;
use ros_z::prelude::*;
use types::{motion_command::MotionCommand, parameters::ImageRegionParameters};

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

    let _parameters = node.bind_parameter_as::<Parameters>("look_at")?;
    let _camera_matrix_sub = node
        .subscriber::<CameraMatrix>("camera_matrix")?
        .build()
        .await?;
    let _ground_to_robot_sub = node
        .subscriber::<Isometry3<Ground, Robot>>("ground_to_robot")?
        .build()
        .await?;
    let _motion_command_sub = node
        .subscriber::<MotionCommand>("motion_command")?
        .build()
        .await?;
    let _serial_motor_states_sub = node
        .subscriber::<Joints<MotorState>>("inputs/serial_motor_states")?
        .build()
        .await?;
    let _look_at_pub = node
        .publisher::<HeadJoints<f32>>("look_at")?
        .build()
        .await?;

    pending::<()>().await;

    Ok(())
}
