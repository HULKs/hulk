use std::{future::pending, sync::Arc, time::Duration};

use color_eyre::Result;
use kinematics::joints::head::HeadJoints;
use ros_z::prelude::*;
use serde::{Deserialize, Serialize};

use crate::IntoEyreResultExt;

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
    // TODO: booster_sdk is not owned by HULKs, we cannot directly implement Message for that...
    // let _robot_mode_sub = node
    //     .subscriber::<RobotMode>("robot_mode")
    //     .build()
    //     .await
    //     .into_eyre()?;
    let _head_joints_sub = node
        .subscriber::<HeadJoints<f32>>("head_joints_command")
        .build()
        .await
        .into_eyre()?;

    pending::<()>().await;

    Ok(())
}
