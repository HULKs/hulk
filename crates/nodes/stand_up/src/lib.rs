use std::{boxed::Box, future::Future, pin::Pin};
use std::{future::pending, sync::Arc};

use color_eyre::Result;

use ros_z::prelude::*;
use types::motion_command::MotionCommand;

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("stand_up").build().await?;

    let _motion_command_sub = node
        .subscriber::<MotionCommand>("motion_command")?
        .build()
        .await?;

    pending::<()>().await;

    Ok(())
}
