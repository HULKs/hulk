use std::{future::pending, sync::Arc};

use color_eyre::Result;
use coordinate_systems::{Field, Ground};
use linear_algebra::{Isometry2, Point2};
use ros_z::{IntoEyreResultExt, prelude::*};
use serde::{Deserialize, Serialize};
use types::{
    field_dimensions::FieldDimensions, filtered_game_controller_state::FilteredGameControllerState,
    obstacles::Obstacle, parameters::LookActionParameters, world_state::BallState,
};

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub field_dimensions: FieldDimensions,
    pub parameters: LookActionParameters,
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("active_vision").build().await.into_eyre()?;

    let _parameters = node
        .bind_parameter_as::<Parameters>("active_vision")
        .into_eyre()?;
    let _ball_sub = node
        .subscriber::<BallState>("ball_state")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _rule_ball_sub = node
        .subscriber::<BallState>("rule_ball_state")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _obstacles_sub = node
        .subscriber::<Vec<Obstacle>>("obstacles")
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
    let _filtered_game_controller_state_sub = node
        .subscriber::<FilteredGameControllerState>("filtered_game_controller_state")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _position_of_interest_pub = node
        .publisher::<Point2<Ground>>("position_of_interest")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;

    pending::<()>().await;

    Ok(())
}
