use std::{future::pending, sync::Arc};

use color_eyre::Result;
use ros_z::prelude::*;
use serde::{Deserialize, Serialize};
use types::{
    field_dimensions::FieldDimensions, filtered_game_controller_state::FilteredGameControllerState,
    rule_obstacles::RuleObstacle, world_state::BallState,
};

use ros_z::IntoEyreResultExt;

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub center_circle_obstacle_radius_increase: f32,
    pub center_circle_ballspace_free_obstacle_radius: f32,
    pub field_dimensions: FieldDimensions,
    pub free_kick_obstacle_radius: f32,
    pub penaltykick_box_extension: f32,
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx
        .create_node("rule_obstacle_composer")
        .build()
        .await
        .into_eyre()?;

    let _parameters = node
        .bind_parameter_as::<Parameters>("rule_obstacle_composer")
        .into_eyre()?;
    let _filtered_game_controller_state_sub = node
        .subscriber::<FilteredGameControllerState>("filtered_game_controller_state")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _ball_state_sub = node
        .subscriber::<BallState>("ball_state")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _rule_obstacles_pub = node
        .publisher::<Vec<RuleObstacle>>("rule_obstacles")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;

    pending::<()>().await;

    Ok(())
}
