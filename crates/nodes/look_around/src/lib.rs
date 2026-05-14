use std::{future::pending, sync::Arc};

use color_eyre::Result;
use serde::{Deserialize, Serialize};

use kinematics::joints::head::HeadJoints;
use ros_z::{IntoEyreResultExt, prelude::*};
use types::{
    filtered_game_controller_state::FilteredGameControllerState,
    initial_look_around::LookAroundMode, motion_command::MotionCommand,
    parameters::LookAroundParameters,
};

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub config: LookAroundParameters,
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("look_around").build().await.into_eyre()?;

    let _parameters = node
        .bind_parameter_as::<Parameters>("look_around")
        .into_eyre()?;
    let _motion_command_sub = node
        .subscriber::<MotionCommand>("motion_command")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _filtered_game_controller_state_sub = node
        .subscriber::<FilteredGameControllerState>("filtered_game_controller_state")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _current_mode_pub = node
        .publisher::<LookAroundMode>("look_around_mode")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _look_around_target_joints_pub = node
        .publisher::<HeadJoints<f32>>("look_around_target_joints")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;

    pending::<()>().await;

    Ok(())
}
