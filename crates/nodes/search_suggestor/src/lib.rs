use std::{future::pending, sync::Arc};

use color_eyre::Result;
use serde::{Deserialize, Serialize};

use coordinate_systems::{Field, Ground};
use linear_algebra::{Isometry2, Point2};
use ros_z::{IntoEyreResultExt, prelude::*};
use types::{
    ball_position::{BallPosition, HypotheticalBallPosition},
    field_dimensions::FieldDimensions,
    filtered_game_controller_state::FilteredGameControllerState,
    messages::IncomingMessage,
    parameters::SearchSuggestorParameters,
    primary_state::PrimaryState,
};

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub field_dimensions: FieldDimensions,
    pub search_suggestor_configuration: SearchSuggestorParameters,
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx
        .create_node("search_suggestor")
        .build()
        .await
        .into_eyre()?;

    let _parameters = node
        .bind_parameter_as::<Parameters>("search_suggestor")
        .into_eyre()?;
    let _ball_position_sub = node
        .subscriber::<BallPosition<Ground>>("ball_position")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _hypothetical_ball_positions_sub = node
        .subscriber::<Vec<HypotheticalBallPosition<Ground>>>("hypothetical_ball_positions")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _ground_to_field_sub = node
        .subscriber::<Isometry2<Ground, Field>>("ground_to_field")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _primary_state_sub = node
        .subscriber::<PrimaryState>("primary_state")
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
    let _network_message_sub = node
        .subscriber::<IncomingMessage>("filtered_message")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    // TODO: do we need to directly publish ndarray here? Choose another type or manually implement
    // support for `Array: Message` in ros-z
    // let _heatmap_pub = node
    //     .publisher::<Array2<f32>>("ball_search_heatmap")
    //     .build()
    //     .await
    //     .into_eyre()?;
    let _suggested_search_position_pub = node
        .publisher::<Point2<Field>>("suggested_search_position")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;

    pending::<()>().await;

    Ok(())
}
