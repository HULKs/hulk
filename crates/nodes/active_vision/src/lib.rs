use std::{future::pending, sync::Arc};

use color_eyre::Result;
use coordinate_systems::{Field, Ground};
use linear_algebra::{Isometry2, Point2};
use ros_z::{prelude::*, qos::QosDurability};
use types::{
    field_dimensions::FieldDimensions, filtered_game_controller_state::FilteredGameControllerState,
    obstacles::Obstacle, parameters::LookActionParameters, world_state::BallState,
};

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("active_vision").build().await?;

    let _parameters = node.bind_parameter_as::<LookActionParameters>("active_vision")?;
    let _field_dimensions_sub = node
        .subscriber::<FieldDimensions>("field_dimensions")?
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .build()
        .await?;
    let _ball_sub = node.subscriber::<BallState>("ball_state")?.build().await?;
    let _rule_ball_sub = node
        .subscriber::<BallState>("rule_ball_state")?
        .build()
        .await?;
    let _obstacles_sub = node
        .subscriber::<Vec<Obstacle>>("obstacles")?
        .build()
        .await?;
    let _ground_to_field_sub = node
        .subscriber::<Isometry2<Ground, Field>>("ground_to_field")?
        .build()
        .await?;
    let _filtered_game_controller_state_sub = node
        .subscriber::<FilteredGameControllerState>("filtered_game_controller_state")?
        .build()
        .await?;
    let _position_of_interest_pub = node
        .publisher::<Point2<Ground>>("position_of_interest")?
        .build()
        .await?;

    pending::<()>().await;

    Ok(())
}
