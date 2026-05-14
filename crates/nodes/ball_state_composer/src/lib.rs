use std::{future::pending, sync::Arc};

use color_eyre::Result;
use serde::{Deserialize, Serialize};

use coordinate_systems::{Field, Ground};
use linear_algebra::Isometry2;
use ros_z::{IntoEyreResultExt, prelude::*};
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
    let node = ctx
        .create_node("ball_state_composer")
        .build()
        .await
        .into_eyre()?;

    let _parameters = node
        .bind_parameter_as::<Parameters>("ball_state_composer")
        .into_eyre()?;
    let _ball_position_sub = node
        .subscriber::<BallPosition<Ground>>("ball_position")
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
    let _team_ball_sub = node
        .subscriber::<BallPosition<Field>>("team_ball")
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
    let _additional_last_ball_state_pub = node
        .publisher::<LastBallState>("last_ball_state")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _ball_state_pub = node
        .publisher::<BallState>("ball_state")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _rule_ball_state_pub = node
        .publisher::<BallState>("rule_ball_state")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;

    pending::<()>().await;

    Ok(())
}
