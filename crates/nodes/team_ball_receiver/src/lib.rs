use std::{boxed::Box, future::Future, pin::Pin};
use std::{future::pending, sync::Arc, time::Duration};

use color_eyre::Result;
use serde::{Deserialize, Serialize};

use coordinate_systems::Field;
use ros_z::prelude::*;
use types::{
    ball_position::BallPosition, filtered_game_controller_state::FilteredGameControllerState,
    messages::IncomingMessage, players::Players,
};

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub maximum_age: Duration,
}

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("team_ball_receiver").build().await?;

    let _parameters = node
        .bind_parameter_as::<Parameters>("team_ball_receiver")
        .await?;
    let _filtered_game_controller_state_sub = node
        .subscriber::<FilteredGameControllerState>("filtered_game_controller_state")?
        .build()
        .await?;
    let _network_message_sub = node
        .subscriber::<IncomingMessage>("filtered_message")?
        .build()
        .await?;
    let _team_balls_pub = node
        .publisher::<Players<Option<BallPosition<Field>>>>("team_balls")?
        .build()
        .await?;
    let _team_ball_pub = node
        .publisher::<BallPosition<Field>>("team_ball")?
        .build()
        .await?;

    pending::<()>().await;

    Ok(())
}
