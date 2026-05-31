use std::{boxed::Box, future::Future, pin::Pin};
use std::{future::pending, sync::Arc, time::Duration};

use color_eyre::Result;

use ros_z::prelude::*;
use types::world_state::BallState;

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx
        .create_node("time_to_reach_kick_position")
        .build()
        .await?;
    let _ball_state_sub = node.subscriber::<BallState>("ball_state")?.build().await?;
    let _time_to_reach_kick_position_pub = node
        .publisher::<Duration>("time_to_reach_kick_position")?
        .build()
        .await?;

    pending::<()>().await;

    Ok(())
}
