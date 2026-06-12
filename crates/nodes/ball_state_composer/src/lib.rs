use std::{boxed::Box, future::Future, pin::Pin};
use std::{sync::Arc, time::SystemTime};

use color_eyre::Result;

use coordinate_systems::{Field, Ground};
use filtering::hysteresis::greater_than_with_hysteresis;
use hsl_network_messages::{GamePhase, SubState, Team};
use linear_algebra::{Isometry2, Point2, Vector2, point};
use ros_z::{prelude::*, qos::QosDurability};
use types::{
    ball_position::BallPosition,
    field_dimensions::{FieldDimensions, Side},
    filtered_game_controller_state::FilteredGameControllerState,
    primary_state::PrimaryState,
    world_state::{BallState, LastBallState},
};

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("ball_state_composer").build().await?;

    let field_dimensions_cache = node
        .create_cache::<FieldDimensions>("field_dimensions", 1)?
        .with_qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .build()
        .await?;
    let ball_position_sub = node
        .subscriber::<Option<BallPosition<Ground>>>("ball_filter/ball_position")?
        .build()
        .await?;
    let ground_to_field_cache = node
        .create_cache::<Isometry2<Ground, Field>>("ground_to_field", 10)?
        .build()
        .await?;
    let team_ball_sub = node
        .subscriber::<BallPosition<Field>>("team_ball")?
        .build()
        .await?;
    let primary_state_cache = node
        .create_cache::<PrimaryState>("primary_state", 10)?
        .with_qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .build()
        .await?;
    let filtered_game_controller_state_sub = node
        .subscriber::<FilteredGameControllerState>("filtered_game_controller_state")?
        .build()
        .await?;
    let additional_last_ball_state_pub = node
        .publisher::<Option<LastBallState>>("last_ball_state")?
        .build()
        .await?;
    let ball_state_pub = node
        .publisher::<Option<BallState>>("ball_state")?
        .build()
        .await?;
    let rule_ball_state_pub = node
        .publisher::<Option<BallState>>("rule_ball_state")?
        .build()
        .await?;

    let mut last_ball_field_side = Side::Left;
    let mut last_ball_state = None;

    loop {
        let now = node.clock().now().to_wallclock();

        additional_last_ball_state_pub
            .publish_if_subscribed(|| async { last_ball_state })
            .await?;

        tokio::select! {
            received_ball_position = ball_position_sub.recv() => {
                let Some(ball_position) = received_ball_position? else {
                    continue;
                };

                let Some(ground_to_field) = ground_to_field_cache.get_latest() else {
                    continue;
                };
                let ground_to_field = *ground_to_field;

                let ball = create_ball_state(
                    ball_position.position,
                    ground_to_field * ball_position.position,
                    ball_position.velocity,
                    ball_position.last_seen.to_wallclock(),
                    &mut last_ball_field_side,
                );
                ball_state_pub.publish(&Some(ball)).await?;
                last_ball_state = Some(LastBallState {
                    time: now,
                    ball,
                });
            }
            received_team_ball = team_ball_sub.recv() => {
                let team_ball = received_team_ball?;

                let Some(ground_to_field) = ground_to_field_cache.get_latest() else {
                    continue;
                };
                let ground_to_field = *ground_to_field;

                let ball = create_ball_state(
                    ground_to_field.inverse() * team_ball.position,
                    team_ball.position,
                    ground_to_field.inverse() * team_ball.velocity,
                    team_ball.last_seen.to_wallclock(),
                    &mut last_ball_field_side,
                );
                ball_state_pub.publish(&Some(ball)).await?;
                last_ball_state = Some(LastBallState {
                    time: now,
                    ball,
                });
            }
            received_filtered_game_controller_state = filtered_game_controller_state_sub.recv() => {
                let filtered_game_controller_state = received_filtered_game_controller_state?;

                let Some(field_dimensions) = field_dimensions_cache.get_latest() else {
                    continue;
                };
                let field_dimensions = *field_dimensions;

                let Some(ground_to_field) = ground_to_field_cache.get_latest() else {
                    continue;
                };
                let ground_to_field = *ground_to_field;

                let Some(primary_state) = primary_state_cache.get_latest() else { continue };
                let rule_ball = compose_rule_ball_state(
                    *primary_state,
                    ground_to_field,
                    Some(&filtered_game_controller_state),
                    field_dimensions,
                    now,
                    &mut last_ball_field_side,
                );

                rule_ball_state_pub.publish(&rule_ball).await?;
            }
        }
    }
}

fn compose_rule_ball_state(
    primary_state: PrimaryState,
    ground_to_field: Isometry2<Ground, Field>,
    filtered_game_controller_state: Option<&FilteredGameControllerState>,
    field_dimensions: FieldDimensions,
    cycle_start_time: SystemTime,
    last_ball_field_side: &mut Side,
) -> Option<BallState> {
    match (primary_state, filtered_game_controller_state) {
        (
            PrimaryState::Ready | PrimaryState::Set,
            Some(FilteredGameControllerState {
                sub_state: Some(SubState::PenaltyKick),
                kicking_team,
                ..
            })
            | Some(FilteredGameControllerState {
                game_phase:
                    GamePhase::PenaltyShootout {
                        kicking_team: Team::Hulks,
                    },
                kicking_team,
                ..
            }),
        ) => {
            let side_factor = match kicking_team {
                Some(Team::Opponent) => -1.0,
                Some(Team::Hulks) => 1.0,
                _ => -1.0,
            };
            let penalty_spot_x =
                field_dimensions.length / 2.0 - field_dimensions.penalty_marker_distance;
            let penalty_spot_location = point![side_factor * penalty_spot_x, 0.0];
            Some(create_ball_state(
                ground_to_field.inverse() * penalty_spot_location,
                penalty_spot_location,
                Vector2::zeros(),
                cycle_start_time,
                last_ball_field_side,
            ))
        }
        (PrimaryState::Ready, _) => Some(create_ball_state(
            ground_to_field.inverse() * Point2::origin(),
            Point2::origin(),
            Vector2::zeros(),
            cycle_start_time,
            last_ball_field_side,
        )),
        _ => None,
    }
}

fn create_ball_state(
    ball_in_ground: Point2<Ground>,
    ball_in_field: Point2<Field>,
    ball_in_ground_velocity: Vector2<Ground>,
    last_seen_ball: SystemTime,
    last_ball_field_side: &mut Side,
) -> BallState {
    let was_in_left_half = *last_ball_field_side == Side::Left;
    let is_in_left_half =
        greater_than_with_hysteresis(was_in_left_half, ball_in_field.y(), 0.0, 0.2);
    let side = if is_in_left_half {
        Side::Left
    } else {
        Side::Right
    };
    *last_ball_field_side = side;
    let field_side = side;
    BallState {
        ball_in_ground,
        ball_in_field,
        ball_in_ground_velocity,
        last_seen_ball,
        field_side,
    }
}
