use std::{boxed::Box, future::Future, pin::Pin};
use std::{future::pending, sync::Arc};

use color_eyre::Result;
use serde::{Deserialize, Serialize};

use ros_z::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub cycler_frequency: f32,
}

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("trigger").build().await?;

    let _parameters = node.bind_parameter_as::<Parameters>("trigger")?;

    pending::<()>().await;

    Ok(())
}
