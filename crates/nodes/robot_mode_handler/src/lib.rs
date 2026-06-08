use std::{boxed::Box, future::Future, future::pending, pin::Pin, sync::Arc};

use color_eyre::Result;
use ros_z::prelude::*;

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let _node = ctx
        .create_node("robot_mode_handler_deprecated")
        .build()
        .await?;
    pending::<()>().await;
    Ok(())
}
