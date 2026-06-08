use std::{boxed::Box, future::Future, pin::Pin};
use std::{future::pending, sync::Arc};

use color_eyre::Result;

use ros_z::prelude::*;
use types::{motion_command::MotionCommand, parameters::BoosterKickingParameters};

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("kick").build().await?;

    let _parameters = node.bind_parameter_as::<BoosterKickingParameters>("kick")?;
    let _motion_command_sub = node
        .subscriber::<MotionCommand>("motion_command")?
        .build()
        .await?;

    pending::<()>().await;

    Ok(())
}
