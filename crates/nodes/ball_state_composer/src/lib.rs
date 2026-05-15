use std::{future::pending, sync::Arc};

use color_eyre::Result;
use serde::{Deserialize, Serialize};

use coordinate_systems::{Field, Ground};
use linear_algebra::Isometry2;
use ros_z::prelude::*;
use types::{
    ball_position::BallPosition,
    field_dimensions::FieldDimensions,
    filtered_game_controller_state::FilteredGameControllerState,
    primary_state::PrimaryState,
    world_state::{BallState, LastBallState},
};

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub field_dimensions: FieldDimensions,
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("ball_state_composer").build().await?;

    let _parameters = node.bind_parameter_as::<Parameters>("ball_state_composer")?;
    let _ball_position_sub = node
        .subscriber::<BallPosition<Ground>>("ball_position")?
        .build()
        .await?;
    let _ground_to_field_sub = node
        .subscriber::<Isometry2<Ground, Field>>("ground_to_field")?
        .build()
        .await?;
    let _team_ball_sub = node
        .subscriber::<BallPosition<Field>>("team_ball")?
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
    let _additional_last_ball_state_pub = node
        .publisher::<LastBallState>("last_ball_state")?
        .build()
        .await?;
    let _ball_state_pub = node.publisher::<BallState>("ball_state")?.build().await?;
    let _rule_ball_state_pub = node
        .publisher::<BallState>("rule_ball_state")?
        .build()
        .await?;

    pending::<()>().await;

    Ok(())
}
