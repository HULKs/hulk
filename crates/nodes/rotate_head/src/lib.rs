use std::{boxed::Box, future::Future, pin::Pin};
use std::{future::pending, sync::Arc, time::Duration};

use color_eyre::Result;
use serde::{Deserialize, Serialize};

use kinematics::joints::head::HeadJoints;
use ros_z::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub rotate_head_message_interval: Duration,
}

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("rotate_head").build().await?;

    let _parameters = node.bind_parameter_as::<Parameters>("rotate_head")?;
    let _head_joints_sub = node
        .subscriber::<HeadJoints<f32>>("head_joints_command")?
        .build()
        .await?;

    pending::<()>().await;

    Ok(())
}
