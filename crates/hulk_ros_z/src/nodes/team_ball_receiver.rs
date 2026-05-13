use std::{future::pending, sync::Arc, time::Duration};

use color_eyre::Result;
use coordinate_systems::Field;
use ros_z::prelude::*;
use serde::{Deserialize, Serialize};
use types::{
    ball_position::BallPosition, filtered_game_controller_state::FilteredGameControllerState,
    messages::IncomingMessage, players::Players,
};

use crate::IntoEyreResultExt;

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub maximum_age: Duration,
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx
        .create_node("team_ball_receiver")
        .build()
        .await
        .into_eyre()?;

    let _parameters = node
        .bind_parameter_as::<Parameters>("team_ball_receiver")
        .into_eyre()?;
    let _filtered_game_controller_state_sub = node
        .subscriber::<FilteredGameControllerState>("filtered_game_controller_state")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _network_message_sub = node
        .subscriber::<IncomingMessage>("filtered_message")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _team_balls_pub = node
        .publisher::<Players<Option<BallPosition<Field>>>>("team_balls")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _team_ball_pub = node
        .publisher::<BallPosition<Field>>("team_ball")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;

    pending::<()>().await;

    Ok(())
}
