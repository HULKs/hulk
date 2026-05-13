use std::{future::pending, sync::Arc};

use booster::FallDownState;
use color_eyre::Result;
use coordinate_systems::{Field, Ground};
use hsl_network_messages::PlayerNumber;
use linear_algebra::{Isometry2, Point2};
use ros_z::prelude::*;
use serde::{Deserialize, Serialize};
use types::{
    ball_position::HypotheticalBallPosition,
    filtered_game_controller_state::FilteredGameControllerState,
    obstacles::Obstacle,
    primary_state::PrimaryState,
    rule_obstacles::RuleObstacle,
    world_state::{BallState, WorldState},
};

use crate::IntoEyreResultExt;

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub player_number: PlayerNumber,
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx
        .create_node("world_state_composer")
        .build()
        .await
        .into_eyre()?;

    let _parameters = node
        .bind_parameter_as::<Parameters>("world_state_composer")
        .into_eyre()?;
    let _fall_down_state_sub = node
        .subscriber::<FallDownState>("fall_down_state")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _ball_sub = node
        .subscriber::<BallState>("ball_state")
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
    let _ground_to_field_sub = node
        .subscriber::<Isometry2<Ground, Field>>("ground_to_field")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _hypothetical_ball_position_sub = node
        .subscriber::<Vec<HypotheticalBallPosition<Ground>>>("hypothetical_ball_positions")
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
    let _position_of_interest_sub = node
        .subscriber::<Point2<Ground>>("position_of_interest")
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
    let _rule_ball_sub = node
        .subscriber::<BallState>("rule_ball_state")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _rule_obstacles_sub = node
        .subscriber::<Vec<RuleObstacle>>("rule_obstacles")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _suggested_search_position_sub = node
        .subscriber::<Point2<Field>>("suggested_search_position")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _world_state_pub = node
        .publisher::<WorldState>("world_state")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;

    pending::<()>().await;

    Ok(())
}
