use std::{future::pending, sync::Arc};

use color_eyre::Result;
use serde::{Deserialize, Serialize};

use coordinate_systems::Field;
use hsl_network_messages::PlayerNumber;
use linear_algebra::Point2;
use ros_z::prelude::*;
use types::{
    field_dimensions::FieldDimensions, filtered_game_controller_state::FilteredGameControllerState,
    filtered_whistle::FilteredWhistle, game_controller_state::GameControllerState,
    parameters::GameStateFilterParameters,
};

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub config: GameStateFilterParameters,
    pub field_dimensions: FieldDimensions,
    pub player_number: PlayerNumber,
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx
        .create_node("game_controller_state_filter")
        .build()
        .await?;

    let _parameters = node.bind_parameter_as::<Parameters>("game_controller_state_filter")?;
    let _filtered_whistle_sub = node
        .subscriber::<FilteredWhistle>("filtered_whistle")?
        .build()
        .await?;
    let _game_controller_state_sub = node
        .subscriber::<GameControllerState>("game_controller_state")?
        .build()
        .await?;
    let _whistle_in_set_ball_position_pub = node
        .publisher::<Point2<Field>>("whistle_in_set_ball_position")?
        .build()
        .await?;
    let _filtered_game_controller_state_pub = node
        .publisher::<FilteredGameControllerState>("filtered_game_controller_state")?
        .build()
        .await?;

    pending::<()>().await;

    Ok(())
}
