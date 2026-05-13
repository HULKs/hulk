use std::{future::pending, sync::Arc};

use color_eyre::Result;
use ros_z::prelude::*;
use serde::{Deserialize, Serialize};
use types::{motion_command::MotionCommand, parameters::BoosterKickingParameters};

use crate::IntoEyreResultExt;

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub booster_kicking_paramters: BoosterKickingParameters,
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("kick").build().await.into_eyre()?;

    let _parameters = node.bind_parameter_as::<Parameters>("kick").into_eyre()?;
    // TODO: booster_sdk is not owned by HULKs, we cannot directly implement Message for that...
    // let _robot_mode_sub = node
    //     .subscriber::<RobotMode>("robot_mode")
    //     .build()
    //     .await
    //     .into_eyre()?;
    let _motion_command_sub = node
        .subscriber::<MotionCommand>("motion_command")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;

    pending::<()>().await;

    Ok(())
}
