use std::{future::pending, sync::Arc, time::Duration};

use color_eyre::Result;
use ros_z::prelude::*;
use types::world_state::BallState;

use crate::IntoEyreResultExt;

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx
        .create_node("time_to_reach_kick_position")
        .build()
        .await
        .into_eyre()?;
    let _ball_state_sub = node
        .subscriber::<BallState>("ball_state")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _time_to_reach_kick_position_pub = node
        .publisher::<Duration>("time_to_reach_kick_position")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;

    pending::<()>().await;

    Ok(())
}
