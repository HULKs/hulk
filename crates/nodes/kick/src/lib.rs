use std::{future::pending, sync::Arc};

use color_eyre::Result;
use serde::{Deserialize, Serialize};

use booster_sdk_interface::GetRobotMode;
use ros_z::prelude::*;
use types::{motion_command::MotionCommand, parameters::BoosterKickingParameters};

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub booster_kicking_paramters: BoosterKickingParameters,
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("kick").build().await?;

    let _parameters = node.bind_parameter_as::<Parameters>("kick")?;
    let _get_robot_mode_client = node
        .create_service_client::<GetRobotMode>("services/get_robot_mode")?
        .build()
        .await?;
    let _motion_command_sub = node
        .subscriber::<MotionCommand>("motion_command")?
        .build()
        .await?;

    pending::<()>().await;

    Ok(())
}
