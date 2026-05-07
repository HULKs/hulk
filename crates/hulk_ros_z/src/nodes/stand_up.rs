use std::{future::pending, sync::Arc};

use color_eyre::Result;
use ros_z::prelude::*;
use types::motion_command::MotionCommand;

use crate::IntoEyreResultExt;

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("stand_up").build().await.into_eyre()?;
    // TODO: booster_sdk is not owned by HULKs, we cannot directly implement Message for that...
    // let _robot_mode_sub = node
    //     .subscriber::<RobotMode>("robot_mode")
    //     .build()
    //     .await
    //     .into_eyre()?;
    let _motion_command_sub = node
        .subscriber::<MotionCommand>("motion_command")
        .build()
        .await
        .into_eyre()?;

    pending::<()>().await;

    Ok(())
}
