use std::{future::pending, sync::Arc, time::Duration};

use color_eyre::Result;
use serde::{Deserialize, Serialize};

use booster_sdk_interface::GetRobotMode;
use ros_z::{IntoEyreResultExt, prelude::*};
use types::{motion_command::MotionCommand, parameters::RLWalkingParameters, step::Step};

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub parameters: RLWalkingParameters,
    pub move_robot_message_interval: Duration,
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("walking").build().await.into_eyre()?;

    let _parameters = node
        .bind_parameter_as::<Parameters>("walking")
        .into_eyre()?;

    let _get_robot_mode_client = node
        .create_service_client::<GetRobotMode>("services/get_robot_mode")
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
    let _step_pub = node
        .publisher::<Step>("additional_outputs/walking_step")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;

    pending::<()>().await;

    Ok(())
}
