use std::{boxed::Box, future::Future, pin::Pin};
use std::{future::pending, sync::Arc, time::Duration};

use color_eyre::Result;
use serde::{Deserialize, Serialize};

use booster_sdk_interface::GetRobotMode;
use ros_z::prelude::*;
use types::{motion_command::MotionCommand, parameters::RLWalkingParameters, step::Step};

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub parameters: RLWalkingParameters,
    pub move_robot_message_interval: Duration,
}

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("walking").build().await?;

    let _parameters = node.bind_parameter_as::<Parameters>("walking").await?;

    let _get_robot_mode_client = node
        .create_service_client::<GetRobotMode>("services/get_robot_mode")?
        .build()
        .await?;
    let _motion_command_sub = node
        .subscriber::<MotionCommand>("motion_command")?
        .build()
        .await?;
    let _step_pub = node
        .publisher::<Step>("additional_outputs/walking_step")?
        .build()
        .await?;

    pending::<()>().await;

    Ok(())
}
