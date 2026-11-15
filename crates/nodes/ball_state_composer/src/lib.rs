use std::{boxed::Box, future::Future, pin::Pin};
use std::{sync::Arc, time::SystemTime};

use color_eyre::Result;

use coordinate_systems::{Field, Ground, Odometry};
use filtering::hysteresis::greater_than_with_hysteresis;
use ground_frames::ground_alignment::GroundAlignment;
use hsl_network_messages::{GamePhase, SubState, Team};
use linear_algebra::{Isometry2, Point2, Pose2, Vector2, point};
use ros_z::{prelude::*, qos::QosDurability, time::Time};
use types::{
    ball_position::BallPosition,
    field_dimensions::{FieldDimensions, Side},
    filtered_game_controller_state::FilteredGameControllerState,
    primary_state::PrimaryState,
    world_state::{BallSource, BallState, LastBallState},
};

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("ball_state_composer").build().await?;

    let field_dimensions_cache = node
        .subscriber::<FieldDimensions>("field_dimensions")
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .cache(1)
        .build()
        .await?;
    let ball_position_sub = node
        .subscriber::<Option<BallPosition<Ground>>>("ball_filter/ball_position")
        .build()
        .await?;
    // Localization stamps its estimates with the time of their image, and the ball filter its
    // balls with the time of their ground frame, which only the source time carries.
    let ground_to_field_sub = node
        .subscriber::<Isometry2<Ground, Field>>("ground_to_field")
        .build()
        .await?;
    let odometry_sub = node
        .subscriber::<Pose2<Odometry>>("inputs/odometry")
        .build()
        .await?;
    let team_ball_sub = node
        .subscriber::<Option<BallPosition<Field>>>("team_ball")
        .build()
        .await?;
    let primary_state_cache = node
        .subscriber::<PrimaryState>("primary_state")
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .cache(10)
        .build()
        .await?;
    let filtered_game_controller_state_sub = node
        .subscriber::<FilteredGameControllerState>("filtered_game_controller_state")
        .build()
        .await?;
    let additional_last_ball_state_pub = node
        .publisher::<Option<LastBallState>>("last_ball_state")
        .build()
        .await?;
    let ball_state_pub = node
        .publisher::<Option<BallState>>("ball_state")
        .build()
        .await?;
    let rule_ball_state_pub = node
        .publisher::<Option<BallState>>("rule_ball_state")
        .build()
        .await?;

    let mut last_ball_field_side = Side::Left;
    let mut last_ball_state = None;
    let mut ball_position = None;
    let mut team_ball = None;
    let mut ground_alignment = GroundAlignment::default();

    'cycle: loop {
        let time = node.clock().now();
        let now = time.to_wallclock();
        ground_alignment.expire_if_silent(time);

        additional_last_ball_state_pub
            .publish_if_subscribed(|| async { last_ball_state })
            .await?;

        loop {
            tokio::select! {
                odometry = odometry_sub.recv_with_metadata() => {
                    let odometry = odometry?;
                    ground_alignment.push_pose(odometry.source_time, odometry.message);
                }
                ground_to_field = ground_to_field_sub.recv_with_metadata() => {
                    let ground_to_field = ground_to_field?;
                    ground_alignment
                        .push_ground_to_field(ground_to_field.source_time, ground_to_field.message);
                }
                received_ball_position = ball_position_sub.recv_with_metadata() => {
                    let received_ball_position = received_ball_position?;
                    ball_position = received_ball_position
                        .message
                        .map(|ball| (received_ball_position.source_time, ball));
                    break;
                }
                received_team_ball = team_ball_sub.recv() => {
                    team_ball = received_team_ball?;
                    break;
                }
                received_filtered_game_controller_state = filtered_game_controller_state_sub.recv() => {
                    let filtered_game_controller_state = received_filtered_game_controller_state?;

                    let Some(field_dimensions) = field_dimensions_cache.get_latest() else {
                        continue 'cycle;
                    };
                    let field_dimensions = *field_dimensions;

                    let Some((frame_time, ground_to_field)) = ground_alignment.current_frame() else {
                        continue 'cycle;
                    };

                    let Some(primary_state) = primary_state_cache.get_latest() else {
                        continue 'cycle;
                    };
                    let rule_ball = compose_rule_ball_state(
                        *primary_state,
                        ground_to_field,
                        Some(&filtered_game_controller_state),
                        field_dimensions,
                        now,
                        &mut last_ball_field_side,
                    );

                    rule_ball_state_pub
                        .publish_with_source_time(&rule_ball, frame_time)
                        .await?;
                    continue 'cycle;
                }
            }
        }

        let frame = ball_frame(&ground_alignment, ball_position);
        let ball = frame.and_then(|(_, ground_to_field)| {
            compose_ball_state(
                ball_position.map(|(_, ball)| ball),
                team_ball,
                ground_to_field,
                &mut last_ball_field_side,
            )
        });
        // Its ground position and velocity only hold together with the time of their frame.
        ball_state_pub
            .publish_with_source_time(&ball, frame.map_or(time, |(frame_time, _)| frame_time))
            .await?;
        last_ball_state = ball.map(|ball| LastBallState { time: now, ball });
    }
}

/// The ground frame a ball state is composed in: its time and where it is in the field.
///
/// A ball is located in the ground frame of its time, so that frame stays its frame and is where it
/// is brought into the field. Without an own ball, a team ball is brought into the current frame.
fn ball_frame(
    ground_alignment: &GroundAlignment,
    ball_position: Option<(Time, BallPosition<Ground>)>,
) -> Option<(Time, Isometry2<Ground, Field>)> {
    match ball_position {
        Some((time, _)) => Some((time, ground_alignment.ground_to_field_at(time)?)),
        None => ground_alignment.current_frame(),
    }
}

fn compose_ball_state(
    ball_position: Option<BallPosition<Ground>>,
    team_ball: Option<BallPosition<Field>>,
    ground_to_field: Isometry2<Ground, Field>,
    last_ball_field_side: &mut Side,
) -> Option<BallState> {
    let (ball_position, ball_in_field, source) = match (ball_position, team_ball) {
        (Some(ball_position), _) => (
            ball_position,
            ground_to_field * ball_position.position,
            BallSource::Own,
        ),
        (None, Some(team_ball)) => (
            ground_to_field.inverse() * team_ball,
            team_ball.position,
            BallSource::Team,
        ),
        (None, None) => return None,
    };

    Some(create_ball_state(
        ball_position.position,
        ball_in_field,
        ball_position.velocity,
        ball_position.last_seen.to_wallclock(),
        source,
        last_ball_field_side,
    ))
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
                BallSource::Rule,
                last_ball_field_side,
            ))
        }
        (PrimaryState::Ready, _) => Some(create_ball_state(
            ground_to_field.inverse() * Point2::origin(),
            Point2::origin(),
            Vector2::zeros(),
            cycle_start_time,
            BallSource::Rule,
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
    source: BallSource,
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
        source,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ready_center_ball_has_rule_source() {
        let mut last_ball_field_side = Side::Left;
        let ball = compose_rule_ball_state(
            PrimaryState::Ready,
            Isometry2::<Ground, Field>::identity(),
            None,
            FieldDimensions::SPL_2025,
            SystemTime::UNIX_EPOCH,
            &mut last_ball_field_side,
        )
        .unwrap();

        assert_eq!(ball.source, BallSource::Rule);
    }
}
