use std::{future::pending, sync::Arc};

use color_eyre::Result;

use coordinate_systems::{Field, Ground};
use linear_algebra::{Isometry2, Point2};
use ros_z::{prelude::*, qos::QosDurability};
use types::{
    ball_position::{BallPosition, HypotheticalBallPosition},
    field_dimensions::FieldDimensions,
    filtered_game_controller_state::FilteredGameControllerState,
    messages::IncomingMessage,
    parameters::SearchSuggestorParameters,
    primary_state::PrimaryState,
};

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("search_suggestor").build().await?;

    let _parameters = node.bind_parameter_as::<SearchSuggestorParameters>("search_suggestor")?;
    let _field_dimensions_sub = node
        .subscriber::<FieldDimensions>("field_dimensions")?
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .build()
        .await?;
    let _ball_position_sub = node
        .subscriber::<BallPosition<Ground>>("ball_position")?
        .build()
        .await?;
    let _hypothetical_ball_positions_sub = node
        .subscriber::<Vec<HypotheticalBallPosition<Ground>>>("hypothetical_ball_positions")?
        .build()
        .await?;
    let _ground_to_field_sub = node
        .subscriber::<Isometry2<Ground, Field>>("ground_to_field")?
        .build()
        .await?;
    let _primary_state_sub = node
        .subscriber::<PrimaryState>("primary_state")?
        .build()
        .await?;
    let _filtered_game_controller_state_sub = node
        .subscriber::<FilteredGameControllerState>("filtered_game_controller_state")?
        .build()
        .await?;
    let _network_message_sub = node
        .subscriber::<IncomingMessage>("filtered_message")?
        .build()
        .await?;
    // TODO: do we need to directly publish ndarray here? Choose another type or manually implement
    // support for `Array: Message` in ros-z
    // let _heatmap_pub = node
    //     .publisher::<Array2<f32>>("ball_search_heatmap")
    //     .build()
    //     .await?;
    let _suggested_search_position_pub = node
        .publisher::<Point2<Field>>("suggested_search_position")?
        .build()
        .await?;

    pending::<()>().await;

    Ok(())
}
