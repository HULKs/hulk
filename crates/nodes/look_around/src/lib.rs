use std::{boxed::Box, future::Future, pin::Pin};
use std::{future::pending, sync::Arc};

use color_eyre::Result;

use kinematics::joints::head::HeadJoints;
use ros_z::prelude::*;
use types::{
    filtered_game_controller_state::FilteredGameControllerState,
    initial_look_around::LookAroundMode, motion_command::MotionCommand,
    parameters::LookAroundParameters,
};

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("look_around").build().await?;

    let _parameters = node.bind_parameter_as::<LookAroundParameters>("look_around")?;
    let _motion_command_sub = node
        .subscriber::<MotionCommand>("motion_command")?
        .build()
        .await?;
    let _filtered_game_controller_state_sub = node
        .subscriber::<Option<FilteredGameControllerState>>("filtered_game_controller_state")?
        .build()
        .await?;
    let _current_mode_pub = node
        .publisher::<LookAroundMode>("look_around_mode")?
        .build()
        .await?;
    let _look_around_target_joints_pub = node
        .publisher::<HeadJoints<f32>>("look_around_target_joints")?
        .build()
        .await?;

    pending::<()>().await;

    Ok(())
}
