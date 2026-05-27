use std::{boxed::Box, future::Future, pin::Pin};
use std::{future::pending, sync::Arc};

use color_eyre::Result;

use booster::FallDownState;
use coordinate_systems::{Field, Ground};
use hsl_network_messages::PlayerNumber;
use linear_algebra::{Isometry2, Point2};
use ros_z::{prelude::*, qos::QosDurability};
use types::{
    ball_position::HypotheticalBallPosition,
    filtered_game_controller_state::FilteredGameControllerState,
    obstacles::Obstacle,
    primary_state::PrimaryState,
    rule_obstacles::RuleObstacle,
    time_wrapper::TimeWrapper,
    world_state::{BallState, WorldState},
};

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("world_state_composer").build().await?;

    let _player_number_sub = node
        .subscriber::<PlayerNumber>("player_number")?
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .build()
        .await?;
    let _fall_down_state_sub = node
        .subscriber::<FallDownState>("inputs/fall_down_state")?
        .build()
        .await?;
    let _ball_sub = node.subscriber::<BallState>("ball_state")?.build().await?;
    let _filtered_game_controller_state_sub = node
        .subscriber::<FilteredGameControllerState>("filtered_game_controller_state")?
        .build()
        .await?;
    let _ground_to_field_sub = node
        .subscriber::<Option<Isometry2<Ground, Field>>>("ground_to_field")?
        .build()
        .await?;
    let _hypothetical_ball_position_sub = node
        .subscriber::<Vec<HypotheticalBallPosition<Ground>>>("hypothetical_ball_positions")?
        .build()
        .await?;
    let _obstacles_sub = node
        .subscriber::<Vec<Obstacle>>("obstacles")?
        .build()
        .await?;
    let _position_of_interest_sub = node
        .subscriber::<Point2<Ground>>("position_of_interest")?
        .build()
        .await?;
    let _primary_state_sub = node
        .subscriber::<PrimaryState>("primary_state")?
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .build()
        .await?;
    let _rule_ball_sub = node
        .subscriber::<BallState>("rule_ball_state")?
        .build()
        .await?;
    let _rule_obstacles_sub = node
        .subscriber::<Vec<RuleObstacle>>("rule_obstacles")?
        .build()
        .await?;
    let _suggested_search_position_sub = node
        .subscriber::<Point2<Field>>("suggested_search_position")?
        .build()
        .await?;
    let _world_state_pub = node.publisher::<WorldState>("world_state")?.build().await?;

    pending::<()>().await;

    Ok(())
}
