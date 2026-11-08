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
    use linear_algebra::vector;

    fn time(milliseconds: i64) -> Time {
        Time::from_nanos((1_000 + milliseconds) * 1_000_000)
    }

    /// The robot turns while a ball rests 3 m ahead of its first pose and localization lags.
    #[test]
    fn resting_ball_stays_in_the_field_while_the_robot_turns() {
        let mut alignment = GroundAlignment::default();
        let pose_at = |milliseconds: i64| {
            Pose2::<Odometry>::new(point![0.0, 0.0], milliseconds as f32 * 1e-3)
        };
        let world_ball = point![<Odometry>, 3.0, 0.0];
        alignment.push_pose(time(0), pose_at(0));
        alignment.push_ground_to_field(time(0), Isometry2::identity());
        let mut side = Side::Left;
        for milliseconds in (2..=200).step_by(2) {
            alignment.push_pose(time(milliseconds), pose_at(milliseconds));
            let ball_time = milliseconds - 26;
            if ball_time < 0 {
                continue;
            }
            let ball = BallPosition {
                position: pose_at(ball_time).as_transform::<Ground>().inverse() * world_ball,
                velocity: vector![0.0, 0.0],
                last_seen: time(ball_time),
            };
            let (frame_time, frame) =
                ball_frame(&alignment, Some((time(ball_time), ball))).unwrap();
            let state = compose_ball_state(Some(ball), None, frame, &mut side).unwrap();
            assert_eq!(frame_time, time(ball_time));
            assert!((state.ball_in_field - point![3.0, 0.0]).norm() < 1e-3);
            if milliseconds % 40 == 0 {
                // Localization of an image 12 ms ago, as if it was the current one.
                let image_time = milliseconds - 12;
                alignment.push_ground_to_field(
                    time(image_time),
                    Isometry2::wrap(pose_at(image_time).as_transform::<Ground>().inner),
                );
            }
        }
    }

    #[test]
    fn ball_state_keeps_the_ground_frame_of_its_ball() {
        let ball = BallPosition {
            position: point![3.0, 0.0],
            velocity: vector![1.0, 0.0],
            last_seen: time(-100),
        };
        // Only the ground frame of the ball tells where it was: the robot has turned since.
        let ground_to_field = Isometry2::from_parts(vector![1.0, 0.0], std::f32::consts::FRAC_PI_2);

        let state = compose_ball_state(Some(ball), None, ground_to_field, &mut Side::Left).unwrap();

        assert_eq!(state.ball_in_ground, ball.position);
        assert_eq!(state.ball_in_ground_velocity, ball.velocity);
        assert!((state.ball_in_field - point![1.0, 3.0]).norm() < 1e-5);
    }

    #[test]
    fn ball_without_a_pose_of_its_time_has_no_field_position() {
        let mut alignment = GroundAlignment::default();
        alignment.push_ground_to_field(time(0), Isometry2::identity());
        alignment.push_pose(time(0), Pose2::zero());
        let ball = BallPosition {
            position: point![3.0, 0.0],
            velocity: vector![0.0, 0.0],
            last_seen: time(50),
        };

        assert!(ball_frame(&alignment, Some((time(50), ball))).is_none());
        assert_eq!(
            ball_frame(&alignment, None).map(|(frame_time, _)| frame_time),
            Some(time(0))
        );
    }

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

#[cfg(test)]
mod node_tests {
    use std::{path::Path, time::Duration};

    use hsl_network_messages::Half;
    use linear_algebra::vector;
    use ros_z_streams::CreateAnnouncingPublisher;
    use types::{
        field_dimensions::GlobalFieldSide, filtered_game_state::FilteredGameState, players::Players,
    };

    use super::*;

    /// Runs the actual node on publications stamped like the ones of its producers, and checks
    /// that each ball state tells the time of the ground frame it is expressed in.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn node_publishes_states_with_the_time_of_their_ground_frame() {
        let context = Arc::new(
            ContextBuilder::default()
                .with_mode("peer")
                .disable_multicast_scouting()
                .with_connect_endpoints(std::iter::empty::<&str>())
                .with_listen_endpoints(std::iter::empty::<&str>())
                .with_parameter_layers([
                    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../etc/parameters/base")
                ])
                .build()
                .await
                .unwrap(),
        );
        let node = context.create_node("fixture").build().await.unwrap();
        let retained = QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        };
        let odometry = node
            .announcing_publisher::<Pose2<Odometry>>("inputs/odometry")
            .await
            .unwrap();
        let localization = node
            .publisher::<Isometry2<Ground, Field>>("ground_to_field")
            .build()
            .await
            .unwrap();
        let balls = node
            .publisher::<Option<BallPosition<Ground>>>("ball_filter/ball_position")
            .build()
            .await
            .unwrap();
        let team_balls = node
            .publisher::<Option<BallPosition<Field>>>("team_ball")
            .build()
            .await
            .unwrap();
        let game = node
            .publisher::<FilteredGameControllerState>("filtered_game_controller_state")
            .build()
            .await
            .unwrap();
        let primary_state = node
            .publisher::<PrimaryState>("primary_state")
            .qos(retained)
            .build()
            .await
            .unwrap();
        let field_dimensions = node
            .publisher::<FieldDimensions>("field_dimensions")
            .qos(retained)
            .build()
            .await
            .unwrap();
        let states = node
            .subscriber::<Option<BallState>>("ball_state")
            .build()
            .await
            .unwrap();
        let rule_states = node
            .subscriber::<Option<BallState>>("rule_ball_state")
            .build()
            .await
            .unwrap();
        primary_state.publish(&PrimaryState::Ready).await.unwrap();
        field_dimensions
            .publish(&FieldDimensions::SPL_2025)
            .await
            .unwrap();
        let task = tokio::spawn(run_boxed(context.clone()));
        tokio::time::timeout(Duration::from_secs(5), async {
            while states.publisher_count() == 0
                || balls.subscriber_count() == 0
                || team_balls.subscriber_count() == 0
                || game.subscriber_count() == 0
                || localization.subscriber_count() == 0
            {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();

        let start = Time::from_nanos(1_000_000_000);
        let turned = start + Duration::from_millis(20);
        let turn = 0.5;
        odometry
            .announce(start)
            .await
            .unwrap()
            .publish(&Pose2::zero())
            .await
            .unwrap();
        localization
            .publish_with_source_time(&Isometry2::identity(), start)
            .await
            .unwrap();
        odometry
            .announce(turned)
            .await
            .unwrap()
            .publish(&Pose2::new(point![0.0, 0.0], turn))
            .await
            .unwrap();
        // Let the node process the poses; they are independent of the ball's stream.
        tokio::time::sleep(Duration::from_millis(100)).await;

        balls
            .publish_with_source_time(
                &Some(BallPosition {
                    position: point![3.0, 0.0],
                    velocity: vector![1.0, 0.0],
                    last_seen: start - Duration::from_millis(500),
                }),
                start,
            )
            .await
            .unwrap();
        let received = tokio::time::timeout(Duration::from_secs(5), states.recv_with_metadata())
            .await
            .unwrap()
            .unwrap();
        let state = received.message.expect("the ball is localized");
        // Located in the ground frame before the turn, the ball must not turn along with it, and
        // the state must say so. Its observation time has nothing to do with this frame.
        assert_eq!(received.source_time, start);
        assert_eq!(state.source, BallSource::Own);
        assert!((state.ball_in_field - point![3.0, 0.0]).norm() < 1e-3);
        assert!((state.ball_in_ground - point![3.0, 0.0]).norm() < 1e-3);
        assert!((state.ball_in_ground_velocity - vector![1.0, 0.0]).norm() < 1e-3);

        balls.publish_with_source_time(&None, turned).await.unwrap();
        let received = tokio::time::timeout(Duration::from_secs(5), states.recv_with_metadata())
            .await
            .unwrap()
            .unwrap();
        assert!(received.message.is_none());
        team_balls
            .publish(&Some(BallPosition {
                position: point![1.0, 2.0],
                velocity: vector![0.0, 0.0],
                last_seen: start,
            }))
            .await
            .unwrap();
        let received = tokio::time::timeout(Duration::from_secs(5), states.recv_with_metadata())
            .await
            .unwrap()
            .unwrap();
        let state = received.message.expect("the team ball is localized");
        let ground_to_field = Isometry2::<Ground, Field>::from_parts(vector![0.0, 0.0], turn);
        assert_eq!(received.source_time, turned);
        assert_eq!(state.source, BallSource::Team);
        assert_eq!(state.ball_in_field, point![1.0, 2.0]);
        assert!((ground_to_field * state.ball_in_ground - point![1.0, 2.0]).norm() < 1e-3);

        game.publish(&FilteredGameControllerState {
            game_state: FilteredGameState::Ready,
            opponent_game_state: FilteredGameState::Ready,
            half: Half::First,
            remaining_time_in_half: Duration::ZERO,
            game_phase: GamePhase::Normal,
            kicking_team: None,
            penalties: Players::new(None),
            remaining_number_of_messages: 0,
            sub_state: None,
            global_field_side: GlobalFieldSide::Home,
            new_own_penalties_last_cycle: Default::default(),
            new_opponent_penalties_last_cycle: Default::default(),
        })
        .await
        .unwrap();
        let received =
            tokio::time::timeout(Duration::from_secs(5), rule_states.recv_with_metadata())
                .await
                .unwrap()
                .unwrap();
        let state = received.message.expect("the rule ball is known in ready");
        assert_eq!(received.source_time, turned);
        assert_eq!(state.source, BallSource::Rule);
        assert!((ground_to_field * state.ball_in_ground - Point2::origin()).norm() < 1e-3);

        task.abort();
        let _ = task.await;
    }
}
