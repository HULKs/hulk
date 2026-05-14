use std::{future::pending, sync::Arc, time::Duration};

use color_eyre::Result;
use serde::{Deserialize, Serialize};

use booster_sdk_interface::GetRobotMode;
use kinematics::joints::head::HeadJoints;
use ros_z::{IntoEyreResultExt, prelude::*};

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub rotate_head_message_interval: Duration,
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("rotate_head").build().await.into_eyre()?;

    let _parameters = node
        .bind_parameter_as::<Parameters>("rotate_head")
        .into_eyre()?;
    let _get_robot_mode_client = node
        .create_service_client::<GetRobotMode>("services/get_robot_mode")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _head_joints_sub = node
        .subscriber::<HeadJoints<f32>>("head_joints_command")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;

    pending::<()>().await;

    Ok(())
}
