use std::{future::pending, sync::Arc};

use color_eyre::Result;

use booster_sdk_interface::GetRobotMode;
use ros_z::{IntoEyreResultExt, prelude::*};
use types::motion_command::MotionCommand;

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("stand_up").build().await.into_eyre()?;

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

    pending::<()>().await;

    Ok(())
}
