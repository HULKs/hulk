use std::time::Duration;

use coordinate_systems::Field;
use geometry::{Distance, line_segment::LineSegment};
use hsl_network_messages::{PlayerNumber, Team};
use linear_algebra::{Orientation2, Point2, Pose2, point};
use types::{
    behavior_tree::Status, filtered_game_state::FilteredGameState, motion_type::MotionType,
    obstacles::ObstacleKind,
};

use crate::{
    action,
    actions::stand,
    behavior_tree::Node,
    condition,
    head::look_at_ball_subtree,
    node::{Blackboard, ball_timeout},
    selection, sequence, subtree,
    switch_motion_type::switch_motion_type,
    walk::walk_to_kickoff_pose,
};

use super::{
    RESTART_RECEIVER_EXTRA_DISTANCE, execution::execute_ball_action, strategy::BallAction,
};

pub(crate) fn kickoff_kicker(blackboard: &Blackboard) -> PlayerNumber {
    if blackboard.parameters.kickoff.pass_to_left {
        PlayerNumber::Three
    } else {
        PlayerNumber::Two
    }
}

pub fn kickoff_receiver(blackboard: &Blackboard) -> Option<PlayerNumber> {
    let candidates = if blackboard.parameters.kickoff.pass_to_left {
        [PlayerNumber::Four, PlayerNumber::Two, PlayerNumber::Five]
    } else {
        [PlayerNumber::Five, PlayerNumber::Three, PlayerNumber::Four]
    };
    candidates.into_iter().find(|&player| {
        player != blackboard.parameters.goalkeeper.player_number
            && player_pose(blackboard, player, Duration::from_secs(2)).is_some()
    })
}

pub fn kickoff_receiver_position(blackboard: &Blackboard) -> Point2<Field> {
    let side = if blackboard.parameters.kickoff.pass_to_left {
        PlayerNumber::Five
    } else {
        PlayerNumber::Four
    };
    let radius = blackboard.parameters.walking.path_planning.robot_radius;
    let y = blackboard.parameters.kickoff.standard_positions[side]
        .position
        .y()
        .abs()
        .max(blackboard.field_dimensions.center_circle_diameter / 2.0 + radius + 0.05)
        + radius
        + blackboard
            .parameters
            .kicking
            .strategy
            .passing
            .receiver_standoff
        + RESTART_RECEIVER_EXTRA_DISTANCE;
    kickoff_support_position(
        blackboard,
        point![
            -0.3,
            if blackboard.parameters.kickoff.pass_to_left {
                y
            } else {
                -y
            }
        ],
    )
}

fn kickoff_pass_position(blackboard: &Blackboard) -> Option<Point2<Field>> {
    let ball = blackboard.ball.as_ref()?.position;
    let staged = kickoff_receiver_position(blackboard);
    let direction = (staged - ball).try_normalize(f32::EPSILON)?;
    Some(
        staged
            - direction
                * blackboard
                    .parameters
                    .kicking
                    .strategy
                    .passing
                    .receiver_standoff,
    )
}

pub(crate) fn kickoff_receiving_pose(blackboard: &Blackboard) -> Option<Pose2<Field>> {
    if !own_kickoff(blackboard) {
        return None;
    }
    let ball = blackboard.ball.as_ref()?.position;
    let target = kickoff_pass_position(blackboard)?;
    let direction = (target - ball).try_normalize(f32::EPSILON)?;
    let position = target
        + direction
            * blackboard
                .parameters
                .kicking
                .strategy
                .passing
                .receiver_standoff;
    // Ready uses the planner margin; the Playing stance retains body clearance plus a ball radius.
    let clearance = blackboard.parameters.walking.path_planning.robot_radius / 2.0
        + blackboard.field_dimensions.ball_radius;
    Some(Pose2::from_parts(
        point![position.x().min(-clearance), position.y()],
        Orientation2::from_vector(-direction),
    ))
}

pub(crate) fn kickoff_support_position(
    blackboard: &Blackboard,
    position: Point2<Field>,
) -> Point2<Field> {
    let field = &blackboard.field_dimensions;
    let clearance = blackboard.parameters.walking.path_planning.robot_radius + 0.05;
    let circle_radius = field.center_circle_diameter / 2.0 + clearance;
    let x_limit = field.length / 2.0 - clearance;
    let y_limit = field.width / 2.0 - clearance;
    let y = position.y().clamp(-y_limit, y_limit);
    let x = position.x().clamp(-x_limit, -clearance);
    let circle_boundary = -(circle_radius.powi(2) - y.powi(2)).max(0.0).sqrt();
    point![x.min(circle_boundary).max(-x_limit), y]
}

fn is_own_kickoff(blackboard: &mut Blackboard) -> bool {
    let receiver = kickoff_receiver(blackboard);
    let receiving_ball = Some(blackboard.player_number) == receiver
        && blackboard.ball.as_ref().is_some_and(|ball| {
            let now = blackboard.world_state.now;
            let speed = ball.velocity.norm();
            ball.age <= now
                && now.duration_since(ball.age) < ball_timeout(&blackboard.parameters, ball.source)
                && ball.position.coords().norm() > blackboard.field_dimensions.ball_radius / 2.0
                && speed.is_finite()
                && speed
                    > blackboard
                        .parameters
                        .ball
                        .interception
                        .minimum_ball_velocity
        });
    let active = own_kickoff(blackboard)
        && !receiving_ball
        && blackboard.has_team_message_budget()
        && blackboard.player_number != blackboard.parameters.goalkeeper.player_number
        && blackboard.parameters.goalkeeper.player_number != kickoff_kicker(blackboard)
        && player_pose(
            blackboard,
            kickoff_kicker(blackboard),
            Duration::from_secs(2),
        )
        .is_some();
    if !active || receiver.is_some() {
        blackboard.kickoff_receiver_wait_started = None;
        return active;
    }
    if blackboard.player_number != kickoff_kicker(blackboard) {
        return false;
    }
    let now = blackboard.world_state.now;
    let since = *blackboard.kickoff_receiver_wait_started.get_or_insert(now);
    now.duration_since(since) < Duration::from_secs(2)
}

pub(crate) fn needs_kickoff_updates(blackboard: &Blackboard) -> bool {
    blackboard.player_number != blackboard.parameters.goalkeeper.player_number
        && blackboard
            .world_state
            .filtered_game_controller_state
            .as_ref()
            .is_some_and(|game| {
                game.kicking_team == Some(Team::Hulks)
                    && game.sub_state.is_none()
                    && (matches!(
                        game.game_state,
                        FilteredGameState::Ready | FilteredGameState::Set
                    ) || own_kickoff(blackboard))
            })
}

pub(super) fn own_kickoff(blackboard: &Blackboard) -> bool {
    blackboard
        .world_state
        .filtered_game_controller_state
        .as_ref()
        .is_some_and(|game| {
            game.kicking_team == Some(Team::Hulks)
                && game.sub_state.is_none()
                && matches!(game.game_state, FilteredGameState::Playing { .. })
                && (matches!(
                    game.game_state,
                    FilteredGameState::Playing { kick_off: true, .. }
                ) || blackboard.kickoff_started.is_some())
        })
        && blackboard
            .ball
            .as_ref()
            .is_some_and(|ball| ball.position.coords().norm() < 0.35)
}

fn receiver_pose(blackboard: &Blackboard, receiver: PlayerNumber) -> Option<Pose2<Field>> {
    if Some(receiver) != kickoff_receiver(blackboard) {
        return None;
    }
    player_pose(
        blackboard,
        receiver,
        blackboard
            .parameters
            .kicking
            .strategy
            .passing
            .maximum_receiver_age,
    )
}

fn player_pose(
    blackboard: &Blackboard,
    player: PlayerNumber,
    maximum_age: Duration,
) -> Option<Pose2<Field>> {
    let world = &blackboard.world_state;
    let game = world.filtered_game_controller_state.as_ref()?;
    if game.penalties[player].is_some() {
        return None;
    }
    if player == blackboard.player_number {
        return world
            .robot
            .ground_to_field
            .map(|transform| transform.as_pose());
    }
    let state = world.player_states[player].as_ref()?;
    (state.time <= world.now && world.now.duration_since(state.time) < maximum_age)
        .then_some(state.inner.pose)
}

/// Kickoff permits a sideways/backwards pass, while retaining receiver and corridor checks.
pub(super) fn kickoff_pass_target(
    blackboard: &Blackboard,
    receiver: PlayerNumber,
) -> Option<Point2<Field>> {
    if !own_kickoff(blackboard) || !blackboard.has_team_message_budget() {
        return None;
    }
    let parameters = &blackboard.parameters.kicking.strategy;
    parameters.validate().ok()?;
    let pose = receiver_pose(blackboard, receiver)?;
    let stance = kickoff_receiving_pose(blackboard)?;
    let walking = blackboard.parameters.walking.walk_and_stand;
    let orientation_error = pose
        .orientation()
        .rotation_to(stance.orientation())
        .angle()
        .abs();
    if (pose.position() - stance.position()).norm()
        > walking
            .target_reached_thresholds
            .x
            .max(blackboard.field_dimensions.ball_radius)
        || !orientation_error.is_finite()
        || orientation_error > walking.target_reached_thresholds.y + walking.hysteresis.y
    {
        return None;
    }
    let position = pose.position();
    let ball = blackboard.ball.as_ref()?.position;
    let delta = position - ball;
    let distance = delta.norm();
    if !distance.is_finite()
        || distance <= parameters.passing.receiver_standoff
        || distance > parameters.passing.maximum_distance
    {
        return None;
    }
    let target = kickoff_pass_position(blackboard)?;
    if (target - position).norm()
        > parameters.passing.receiver_standoff + 2.0 * parameters.passing.target_half_width
        || (target - ball).norm() > parameters.passing.maximum_distance
    {
        return None;
    }
    let field = &blackboard.field_dimensions;
    if !field.length.is_finite()
        || !field.width.is_finite()
        || target.x().abs() >= field.length / 2.0
        || target.y().abs() >= field.width / 2.0
    {
        return None;
    }
    let transform = blackboard.world_state.robot.ground_to_field?;
    let path = LineSegment(ball, target);
    blackboard
        .world_state
        .obstacles
        .iter()
        .filter(|obstacle| obstacle.kind != ObstacleKind::Ball)
        .all(|obstacle| {
            path.distance_to(transform * obstacle.position)
                > obstacle.radius_at_foot_height + parameters.obstacle_clearance + field.ball_radius
        })
        .then_some(target)
}

fn is_striker_with_receiver(blackboard: &mut Blackboard) -> bool {
    blackboard.player_number == kickoff_kicker(blackboard) && blackboard.has_team_message_budget()
}

fn is_other_field_player(blackboard: &mut Blackboard) -> bool {
    let player = blackboard.player_number;
    player != kickoff_kicker(blackboard)
        && player != blackboard.parameters.goalkeeper.player_number
        && Some(player) != kickoff_receiver(blackboard)
}

fn is_kickoff_receiver(blackboard: &mut Blackboard) -> bool {
    Some(blackboard.player_number) == kickoff_receiver(blackboard)
        && blackboard.player_number != blackboard.parameters.goalkeeper.player_number
}

fn select_kickoff_pass(blackboard: &mut Blackboard) -> Status {
    blackboard.ball_action = None;
    blackboard.kick_target = None;
    let Some(receiver) = kickoff_receiver(blackboard) else {
        return Status::Failure;
    };
    if kickoff_pass_target(blackboard, receiver).is_none() {
        return Status::Failure;
    }
    blackboard.ball_action = Some(BallAction::Pass { receiver });
    Status::Success
}

pub fn kickoff_subtree() -> Node<Blackboard> {
    sequence!(
        condition!(is_own_kickoff),
        subtree!(look_at_ball_subtree),
        selection!(
            sequence!(
                condition!(is_striker_with_receiver),
                selection!(
                    sequence!(action!(select_kickoff_pass), action!(execute_ball_action)),
                    action!(stand),
                ),
            ),
            sequence!(
                selection!(
                    condition!(is_kickoff_receiver),
                    condition!(is_other_field_player)
                ),
                switch_motion_type(
                    MotionType::Walk,
                    action!(walk_to_kickoff_pose),
                    action!(stand)
                ),
            ),
        ),
    )
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use hsl_network_messages::{HulkMessage, Penalty};
    use linear_algebra::{Isometry2, Pose2, vector};
    use types::{
        fall_detection::{FallDetection, Posture},
        field_dimensions::{FieldDimensions, Side},
        filtered_game_controller_state::FilteredGameControllerState,
        messages::OutgoingMessage,
        motion_command::{BodyMotion, HeadMotion, MotionCommand},
        obstacles::Obstacle,
        path::PathSegment,
        primary_state::PrimaryState,
        time_wrapper::TimeWrapper,
        world_state::{BallSource, BallState, PlayerState},
    };

    use crate::node::LastBall;

    use super::*;

    fn blackboard() -> Blackboard {
        let mut board = Blackboard {
            field_dimensions: FieldDimensions::SPL_2025,
            ball: Some(LastBall {
                position: Point2::origin(),
                velocity: Default::default(),
                age: Default::default(),
                field_side: Side::Right,
                source: BallSource::Own,
            }),
            ..Default::default()
        };
        board.parameters.goalkeeper.player_number = PlayerNumber::One;
        board.parameters.kicking.allow_strong_kicks = true;
        board.player_number = PlayerNumber::Two;
        board.world_state.robot.ground_to_field = Some(Default::default());
        board.world_state.fall_detection = Some(FallDetection {
            time: board.world_state.now,
            valid_until: board.world_state.now + Duration::from_secs(60),
            posture: Posture::Upright,
        });
        board.world_state.filtered_game_controller_state = Some(FilteredGameControllerState {
            game_state: FilteredGameState::Playing {
                ball_is_free: true,
                kick_off: true,
            },
            kicking_team: Some(Team::Hulks),
            remaining_number_of_messages: 100,
            remaining_time_in_half: Duration::from_secs(1200),
            ..crate::test_support::filtered_game_controller_state()
        });
        board.parameters.kickoff.standard_positions.four.position = point![-2.5, -1.5];
        board.parameters.kickoff.standard_positions.five.position = point![-2.5, 1.5];
        board
            .parameters
            .walking
            .walk_and_stand
            .target_reached_thresholds = nalgebra::vector![0.05, 0.05];
        board.parameters.walking.speed.velocity_fade_distance = 3.0;
        board.parameters.kicking.strategy = types::parameters::KickStrategyParameters {
            maximum_shot_distance: 3.0,
            maximum_dribble_distance: 1.0,
            obstacle_clearance: 0.1,
            minimum_shot_angle: 0.02,
            preferred_shot_angle: 0.0,
            shot_angle_hysteresis: 0.05,
            pressure_distance: 0.8,
            clear_goal_distance: 1.5,
            clear_distance_hysteresis: 0.2,
            passing: types::parameters::PassingParameters {
                maximum_distance: 3.0,
                minimum_distance: 0.0,
                minimum_progress: 0.5,
                receiver_standoff: 0.5,
                target_half_width: 0.2,
                selection_hysteresis: 0.25,
                maximum_receiver_age: Duration::from_millis(500),
            },
        };
        for receiver in [PlayerNumber::Five, PlayerNumber::Four] {
            board.parameters.kickoff.pass_to_left = receiver == PlayerNumber::Four;
            let pose = kickoff_receiving_pose(&board).unwrap();
            board.world_state.player_states[receiver] = Some(TimeWrapper {
                time: board.world_state.now,
                inner: PlayerState {
                    pose,
                    ..crate::test_support::player_state()
                },
            });
        }
        board.parameters.kickoff.pass_to_left = false;
        board
    }

    #[test]
    fn kickoff_waits_for_fresh_receiver_and_announces_before_adaptive_cooldown() {
        let mut board = blackboard();
        board.player_number = PlayerNumber::Two;
        board.world_state.robot.primary_state = types::primary_state::PrimaryState::Playing;
        board.parameters.kicking.kick_activation_distance = 1.3;
        board
            .world_state
            .filtered_game_controller_state
            .as_mut()
            .unwrap()
            .remaining_time_in_half = Duration::from_secs(1200);
        board.last_sent_hsl_message_time = Some(board.world_state.now);
        assert_eq!(select_kickoff_pass(&mut board), Status::Success);
        assert_eq!(execute_ball_action(&mut board), Status::Success);
        assert_eq!(
            board.body_motion,
            Some(types::motion_command::BodyMotion::Stand)
        );
        board.world_state.now = board.world_state.now + Duration::from_millis(251);
        assert!(board.try_sending_state_message().is_some());
        board.world_state.now = board.world_state.now + Duration::from_millis(150);
        assert_eq!(execute_ball_action(&mut board), Status::Success);
        assert!(matches!(
            board.body_motion,
            Some(types::motion_command::BodyMotion::Kick {
                soft: false,
                strong: true,
                ..
            })
        ));

        // A missing update delays the side pass rather than starting an ordinary shot.
        board.world_state.player_states.five = None;
        board.world_state.player_states.four = None;
        let (status, _) = kickoff_subtree().tick_with_trace(&mut board);
        assert_eq!(status, Status::Success);
        assert_eq!(
            board.body_motion,
            Some(types::motion_command::BodyMotion::Stand)
        );
        assert_eq!(board.ball_action, None);
        board.world_state.now = board.world_state.now + Duration::from_secs(2);
        let (status, _) = kickoff_subtree().tick_with_trace(&mut board);
        assert_eq!(status, Status::Failure);
        board.player_number = PlayerNumber::Five;
        assert!(!is_own_kickoff(&mut board));
        board
            .world_state
            .filtered_game_controller_state
            .as_mut()
            .unwrap()
            .remaining_number_of_messages = 0;
        assert!(board.try_sending_state_message().is_none());
    }

    #[test]
    fn side_parameter_mirrors_wider_legal_receiver_and_backward_pass() {
        let mut board = blackboard();
        board.field_dimensions.length = 14.0;
        board.field_dimensions.width = 9.0;
        board.field_dimensions.center_circle_diameter = 3.0;
        board.parameters.walking.path_planning.robot_radius = 0.4;
        board.parameters.kicking.strategy.passing.maximum_distance = 8.0;
        let right_position = kickoff_receiver_position(&board);
        let minimum_y = board.field_dimensions.center_circle_diameter / 2.0
            + 2.0 * board.parameters.walking.path_planning.robot_radius
            + 0.05
            + board.parameters.kicking.strategy.passing.receiver_standoff
            + 0.5;
        assert!(right_position.y().abs() >= minimum_y - 1e-5);
        let right_stance = kickoff_receiving_pose(&board).unwrap();
        board
            .world_state
            .player_states
            .five
            .as_mut()
            .unwrap()
            .inner
            .pose = right_stance;
        assert_eq!(kickoff_receiver(&board), Some(PlayerNumber::Five));
        let right = kickoff_pass_target(&board, PlayerNumber::Five).unwrap();
        assert!(right.x() < 0.0 && right.y() < 0.0);
        assert!((right - right_position).norm() <= 0.9);
        let incoming = (right - Point2::origin()).normalize();
        assert!(
            (right_position
                - right
                - incoming * board.parameters.kicking.strategy.passing.receiver_standoff)
                .norm()
                < 1e-5
        );
        board.parameters.kickoff.pass_to_left = true;
        let left_position = kickoff_receiver_position(&board);
        assert_eq!(left_position.y(), -right_position.y());
        let left_stance = kickoff_receiving_pose(&board).unwrap();
        board
            .world_state
            .player_states
            .four
            .as_mut()
            .unwrap()
            .inner
            .pose = left_stance;
        assert_eq!(kickoff_receiver(&board), Some(PlayerNumber::Four));
        let left = kickoff_pass_target(&board, PlayerNumber::Four).unwrap();
        assert_eq!(left.x(), right.x());
        assert_eq!(left.y(), -right.y());
        assert!(kickoff_pass_target(&board, PlayerNumber::Five).is_none());

        board.parameters.kickoff.standard_positions.five.position = point![-2.5, 10.0];
        let bounded = kickoff_receiver_position(&board);
        let radius = board.parameters.walking.path_planning.robot_radius;
        assert!(bounded.x() + radius < 0.0);
        assert!(bounded.y().abs() + radius < board.field_dimensions.width / 2.0);
        assert!(
            bounded.coords().norm() - radius > board.field_dimensions.center_circle_diameter / 2.0
        );
    }

    #[test]
    fn receiver_releases_kickoff_after_fresh_ball_motion_before_striker_does() {
        let mut board = blackboard();
        board.world_state.player_states.two = Some(TimeWrapper {
            time: board.world_state.now,
            inner: crate::test_support::player_state(),
        });
        board.player_number = PlayerNumber::Five;
        board.parameters.ball.last_ball_timeout = Duration::from_millis(500);
        board.parameters.ball.interception.minimum_ball_velocity = 0.3;
        let now = board.world_state.now;
        let ball = board.ball.as_mut().unwrap();
        ball.age = now;
        ball.velocity = linear_algebra::vector![0.0, -2.4];
        ball.position = point![0.0, -board.field_dimensions.ball_radius / 4.0];
        assert!(is_own_kickoff(&mut board));

        board.ball.as_mut().unwrap().position = point![0.0, -0.1];
        assert!(own_kickoff(&board));
        assert!(!is_own_kickoff(&mut board));
        board.player_number = PlayerNumber::Two;
        assert!(is_own_kickoff(&mut board));
        board.player_number = PlayerNumber::Five;
        board.ball.as_mut().unwrap().velocity = linear_algebra::vector![f32::INFINITY, 0.0];
        assert!(is_own_kickoff(&mut board));
        board.ball.as_mut().unwrap().velocity = linear_algebra::vector![0.0, -2.4];
        board.world_state.now = now + Duration::from_millis(501);
        assert!(is_own_kickoff(&mut board));
    }

    #[test]
    fn released_kickoff_receiver_shoots_with_observed_pair_and_outgoing_direction() {
        let mut board = blackboard();
        board.player_number = kickoff_receiver(&board).unwrap();
        board.last_motion_type = Some(MotionType::Walk);
        board.parameters.ball.last_ball_timeout = Duration::from_secs(1);
        board.parameters.ball.interception.minimum_ball_velocity = 0.3;
        board.parameters.kicking.kick_activation_distance = 1.3;
        board.parameters.kicking.strategy.maximum_shot_distance = 12.0;
        let position = point![0.4, 0.05];
        let velocity = vector![-0.2, 0.1];
        for side in [-1.0, 1.0] {
            let transform = Isometry2::from_parts(vector![-0.3, side * 2.0], -side * 0.35);
            board.world_state.robot.ground_to_field = Some(transform);
            // The cached field estimate still belongs to the preceding receiving pose.
            board.ball.as_mut().unwrap().position = point![1.2, 0.8];
            board.ball.as_mut().unwrap().velocity = vector![3.0, 0.0];
            board.world_state.ball = Some(BallState {
                ball_in_ground: position,
                ball_in_ground_velocity: velocity,
                ball_in_field: transform * position,
                last_seen_ball: board.world_state.now.to_wallclock(),
                field_side: Side::Left,
                source: BallSource::Own,
            });
            assert!(!is_own_kickoff(&mut board));
            board.ball_action = Some(BallAction::ShootAtGoalCenter);
            assert_eq!(execute_ball_action(&mut board), Status::Success);
            let Some(BodyMotion::Kick {
                ball_position,
                ball_velocity,
                kick_direction,
                strong: true,
                soft: false,
                ..
            }) = board.body_motion
            else {
                panic!("expected receiver's outgoing goal shot")
            };
            assert_eq!(ball_position, position);
            assert_eq!(ball_velocity, velocity);
            let outgoing = Orientation2::from_vector(
                transform.inverse() * point![board.field_dimensions.length / 2.0, 0.0] - position,
            );
            assert!(kick_direction.rotation_to(outgoing).angle().abs() < 1e-6);
        }
    }

    #[test]
    fn available_receiver_prepares_during_ready_and_gets_kickoff_pass() {
        let mut striker = blackboard();
        striker.field_dimensions = FieldDimensions {
            length: 14.0,
            width: 9.0,
            center_circle_diameter: 3.0,
            ball_radius: 0.105,
            ..FieldDimensions::SPL_2025
        };
        striker.parameters.walking.path_planning.robot_radius = 0.4;
        striker.parameters.walking.walk_and_stand.hysteresis = nalgebra::vector![0.3, 0.3];
        striker.parameters.kicking.strategy.passing.maximum_distance = 8.0;
        striker.player_number = PlayerNumber::Two;
        striker.world_state.robot.primary_state = PrimaryState::Ready;
        striker
            .world_state
            .filtered_game_controller_state
            .as_mut()
            .unwrap()
            .game_state = FilteredGameState::Ready;
        striker.world_state.player_states.five = None;
        striker.world_state.player_states.four = None;
        striker.world_state.player_states.three = Some(TimeWrapper {
            time: striker.world_state.now,
            inner: PlayerState {
                pose: Pose2::new(point![-3.0, -0.5], 0.0),
                ..crate::test_support::player_state()
            },
        });
        striker.update_kickoff();
        let mut receiver = striker.clone();
        receiver.player_number = PlayerNumber::Three;
        receiver.world_state.robot.ground_to_field =
            Some(Pose2::new(point![-3.0, -0.5], 0.0).as_transform());
        receiver.world_state.player_states.two = Some(TimeWrapper {
            time: receiver.world_state.now,
            inner: crate::test_support::player_state(),
        });
        receiver.ball = None;
        assert_eq!(kickoff_receiver(&striker), Some(PlayerNumber::Three));
        assert_eq!(kickoff_receiver(&receiver), Some(PlayerNumber::Three));

        let tree = crate::tree::create_tree();
        receiver.update_kickoff();
        assert_eq!(tree.tick_with_trace(&mut receiver).0, Status::Success);
        let Some(BodyMotion::Walk { ref path, .. }) = receiver.body_motion else {
            panic!("kickoff receiver should walk during Ready");
        };
        let PathSegment::LineSegment(segment) = path.last_segment() else {
            panic!("expected straight final approach");
        };
        let target = kickoff_receiver_position(&receiver);
        assert!(target.coords().norm() - 0.4 > 1.5);
        assert!(target.y().abs() >= 1.5 + 2.0 * 0.4 + 0.05 - 1e-5);
        assert!(target.x() + 0.4 < 0.0);
        let actual = receiver.world_state.robot.ground_to_field.unwrap() * segment.1;
        assert!((actual - target).norm() < 0.001);
        assert!(kickoff_pass_target(&striker, PlayerNumber::Three).is_none());

        // Ready discovers available teammates without publishing an old pass intent.
        receiver.ball_action = Some(BallAction::Pass {
            receiver: PlayerNumber::Five,
        });
        receiver.kick_target = Some(point![1.0, 1.0]);
        let Some(OutgoingMessage::Hsl(HulkMessage::State(message))) =
            receiver.try_sending_state_message()
        else {
            panic!("receiver must announce its Ready position");
        };
        assert_eq!(message.pass, None);

        receiver.world_state.now = receiver.world_state.now + Duration::from_millis(251);
        receiver.world_state.robot.primary_state = PrimaryState::Set;
        receiver
            .world_state
            .filtered_game_controller_state
            .as_mut()
            .unwrap()
            .game_state = FilteredGameState::Set;
        receiver.update_kickoff();
        assert!(receiver.try_sending_state_message().is_some());

        let incoming_angle = (-target.y()).atan2(-target.x());
        let outgoing_angle =
            (-target.y()).atan2(receiver.field_dimensions.length / 2.0 - target.x());
        let pose = Pose2::new(target, (incoming_angle + outgoing_angle) / 2.0);
        receiver.world_state.robot.ground_to_field = Some(pose.as_transform());
        receiver.world_state.robot.primary_state = PrimaryState::Playing;
        receiver
            .world_state
            .filtered_game_controller_state
            .as_mut()
            .unwrap()
            .game_state = FilteredGameState::Playing {
            ball_is_free: true,
            kick_off: false,
        };
        receiver.ball = striker.ball.clone();
        receiver.ball_action = None;
        receiver.kick_target = None;
        receiver.update_kickoff();
        assert_eq!(tree.tick_with_trace(&mut receiver).0, Status::Success);
        let receiving_pose = kickoff_receiving_pose(&receiver).unwrap();
        let body_radius = receiver.parameters.walking.path_planning.robot_radius / 2.0;
        assert!((receiving_pose.position() - target).norm() < 1e-5);
        assert!(
            receiving_pose.position().x() + body_radius + receiver.field_dimensions.ball_radius
                <= f32::EPSILON
        );
        assert!(receiving_pose.position().coords().norm() - body_radius > 1.5);
        let Some(BodyMotion::Walk { ref path, .. }) = receiver.body_motion else {
            panic!("receiver must align its Playing stance before the kick");
        };
        let PathSegment::LineSegment(segment) = path.last_segment() else {
            panic!("expected direct approach to receiving stance");
        };
        let actual = receiver.world_state.robot.ground_to_field.unwrap() * segment.1;
        assert!((actual - receiving_pose.position()).norm() < 0.001);

        striker.world_state.now = receiver.world_state.now;
        striker.world_state.robot.primary_state = PrimaryState::Playing;
        striker
            .world_state
            .filtered_game_controller_state
            .as_mut()
            .unwrap()
            .game_state = FilteredGameState::Playing {
            ball_is_free: true,
            kick_off: false,
        };
        striker.world_state.ball = Some(types::world_state::BallState::new_at_center(
            Default::default(),
        ));
        striker.world_state.player_states.three = Some(TimeWrapper {
            time: striker.world_state.now,
            inner: PlayerState {
                pose,
                ..crate::test_support::player_state()
            },
        });
        striker.parameters.kicking.kick_activation_distance = 1.3;
        striker.update_kickoff();
        assert_eq!(tree.tick_with_trace(&mut striker).0, Status::Success);
        assert_eq!(striker.ball_action, None);
        assert_eq!(striker.body_motion, Some(BodyMotion::Stand));

        receiver.world_state.robot.ground_to_field = Some(receiving_pose.as_transform());
        assert_eq!(tree.tick_with_trace(&mut receiver).0, Status::Success);
        assert_eq!(receiver.body_motion, Some(BodyMotion::Stand));
        let settled_pose = Pose2::new(
            receiving_pose.position(),
            receiving_pose.orientation().angle() - 0.16,
        );
        receiver.world_state.robot.ground_to_field = Some(settled_pose.as_transform());
        receiver.last_motion_command = Some(MotionCommand::Stand {
            head: HeadMotion::LookAround,
        });
        assert_eq!(tree.tick_with_trace(&mut receiver).0, Status::Success);
        assert_eq!(receiver.body_motion, Some(BodyMotion::Stand));
        striker
            .world_state
            .player_states
            .three
            .as_mut()
            .unwrap()
            .inner
            .pose = settled_pose;
        let pass_target = kickoff_pass_target(&striker, PlayerNumber::Three).unwrap();
        assert!(pass_target.x() < 0.0);
        let passing = &striker.parameters.kicking.strategy.passing;
        assert!(
            (pass_target - target).norm()
                <= passing.receiver_standoff + 2.0 * passing.target_half_width
        );
        assert_eq!(tree.tick_with_trace(&mut striker).0, Status::Success);
        assert_eq!(
            striker.ball_action,
            Some(BallAction::Pass {
                receiver: PlayerNumber::Three
            })
        );
        assert_eq!(striker.body_motion, Some(BodyMotion::Stand));
        assert!(striker.try_sending_state_message().is_some());
        striker.world_state.now = striker.world_state.now + Duration::from_millis(150);
        striker.update_kickoff();
        assert_eq!(tree.tick_with_trace(&mut striker).0, Status::Success);
        assert!(matches!(
            striker.body_motion,
            Some(BodyMotion::Kick {
                soft: false,
                strong: true,
                ..
            })
        ));
    }

    #[test]
    fn middle_field_staging_keeps_every_non_kicker_outside_center_circle() {
        let tree = crate::tree::create_tree();
        for team in [Team::Hulks, Team::Opponent] {
            for player in [PlayerNumber::Three, PlayerNumber::Two, PlayerNumber::Four] {
                let mut board = blackboard();
                board.field_dimensions.length = 14.0;
                board.field_dimensions.width = 9.0;
                board.field_dimensions.center_circle_diameter = 3.0;
                board.parameters.walking.path_planning.robot_radius = 0.4;
                board.parameters.kickoff.standard_positions[player].position = point![-0.3, 0.2];
                board.parameters.kickoff.striker_position = point![-1.0, 0.0];
                board.player_number = player;
                board.world_state.robot.primary_state = PrimaryState::Ready;
                let game = board
                    .world_state
                    .filtered_game_controller_state
                    .as_mut()
                    .unwrap();
                game.game_state = FilteredGameState::Ready;
                game.kicking_team = Some(team);
                assert_eq!(tree.tick_with_trace(&mut board).0, Status::Success);
                let Some(BodyMotion::Walk { ref path, .. }) = board.body_motion else {
                    panic!("player should walk to kickoff staging");
                };
                let PathSegment::LineSegment(segment) = path.last_segment() else {
                    panic!("expected straight final approach");
                };
                let target = segment.1;
                if team == Team::Hulks && player == PlayerNumber::Two {
                    assert_eq!(target, point![-1.0, 0.0]);
                } else {
                    assert!(target.coords().norm() - 0.4 > 1.5);
                    assert!(target.x() + 0.4 < 0.0);
                    assert!(target.x().abs() + 0.4 < 7.0);
                    assert!(target.y().abs() + 0.4 < 4.5);
                }
            }
        }
        let mut board = blackboard();
        board.field_dimensions.center_circle_diameter = 3.0;
        board.parameters.walking.path_planning.robot_radius = 0.4;
        let right = kickoff_receiver_position(&board);
        board.parameters.kickoff.pass_to_left = true;
        let left = kickoff_receiver_position(&board);
        assert_eq!(left.x(), right.x());
        assert_eq!(left.y(), -right.y());
    }

    #[test]
    fn own_kickoff_swaps_entry_roles_without_waypoints_or_crossing() {
        let tree = crate::tree::create_tree();
        for count in [3, 5] {
            for pass_to_left in [false, true] {
                let mut board = blackboard();
                board.field_dimensions.length = 14.0;
                board.field_dimensions.width = 9.0;
                board.field_dimensions.center_circle_diameter = 3.0;
                board.parameters.walking.path_planning.robot_radius = 0.4;
                board.parameters.kickoff.pass_to_left = pass_to_left;
                board.parameters.kickoff.striker_position = point![-1.5, 0.0];
                board.parameters.kickoff.standard_positions.one.position = point![-4.2, 0.0];
                board.parameters.kickoff.relative_goalkeeper_position =
                    point![-4.2 / (board.field_dimensions.length / 2.0), 0.0];
                board.parameters.kickoff.standard_positions.two.position = point![-3.0, -0.5];
                board.parameters.kickoff.standard_positions.three.position = point![-2.0, 0.0];
                board.world_state.robot.primary_state = PrimaryState::Ready;
                board
                    .world_state
                    .filtered_game_controller_state
                    .as_mut()
                    .unwrap()
                    .game_state = FilteredGameState::Ready;
                board.world_state.player_states = Default::default();
                let starts = [
                    (PlayerNumber::One, point![-6.5, -4.5]),
                    (PlayerNumber::Two, point![-6.5, 4.5]),
                    (PlayerNumber::Three, point![-5.6, -4.5]),
                    (PlayerNumber::Four, point![-5.6, 4.5]),
                    (PlayerNumber::Five, point![-4.7, -4.5]),
                ];
                for &(player, position) in &starts[..count] {
                    board.world_state.player_states[player] = Some(TimeWrapper {
                        time: board.world_state.now,
                        inner: PlayerState {
                            pose: Pose2::new(position, 0.0),
                            ..crate::test_support::player_state()
                        },
                    });
                }
                let expected_receiver = match (count, pass_to_left) {
                    (3, false) => PlayerNumber::Three,
                    (3, true) => PlayerNumber::Two,
                    (_, false) => PlayerNumber::Five,
                    (_, true) => PlayerNumber::Four,
                };
                assert_eq!(kickoff_receiver(&board), Some(expected_receiver));
                assert_eq!(
                    kickoff_kicker(&board),
                    if pass_to_left {
                        PlayerNumber::Three
                    } else {
                        PlayerNumber::Two
                    }
                );
                let mut destinations = Vec::new();
                for &(player, position) in &starts[..count] {
                    board.player_number = player;
                    board.world_state.robot.ground_to_field =
                        Some(Pose2::new(position, 0.0).as_transform());
                    assert_eq!(tree.tick_with_trace(&mut board).0, Status::Success);
                    let Some(BodyMotion::Walk { ref path, .. }) = board.body_motion else {
                        panic!("player must walk directly to its Ready pose");
                    };
                    assert_eq!(path.segments.len(), 1);
                    let PathSegment::LineSegment(segment) = path.last_segment() else {
                        panic!("expected direct Ready route");
                    };
                    let target = board.world_state.robot.ground_to_field.unwrap() * segment.1;
                    if player == kickoff_kicker(&board) {
                        assert!(
                            (target - board.parameters.kickoff.striker_position).norm() < 0.001
                        );
                    } else if player == expected_receiver {
                        assert!((target - kickoff_receiver_position(&board)).norm() < 0.001);
                    } else if player != PlayerNumber::One {
                        assert!(target.coords().norm() - 0.4 > 1.5);
                    }
                    destinations.push(target);
                }
                // Equal-speed entry from actual simulator spawns must leave body clearance.
                for step in 0..=120 {
                    let travel = step as f32 * 0.125;
                    let positions: Vec<_> = starts[..count]
                        .iter()
                        .zip(&destinations)
                        .map(|((_, start), target)| {
                            let delta = *target - *start;
                            *start + delta * (travel / delta.norm()).min(1.0)
                        })
                        .collect();
                    for (index, position) in positions.iter().enumerate() {
                        for other in &positions[index + 1..] {
                            assert!(
                                (*position - *other).norm() > 0.8,
                                "count {count}, left {pass_to_left}, step {step}"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn receiver_selection_excludes_goalkeeper_penalties_and_missing_players() {
        let mut board = blackboard();
        board.player_number = PlayerNumber::Two;
        board.world_state.player_states.three = board.world_state.player_states.five.clone();
        board
            .world_state
            .filtered_game_controller_state
            .as_mut()
            .unwrap()
            .penalties
            .five = Some(Penalty::default());
        assert_eq!(kickoff_receiver(&board), Some(PlayerNumber::Three));
        board.parameters.goalkeeper.player_number = PlayerNumber::Three;
        assert_eq!(kickoff_receiver(&board), Some(PlayerNumber::Four));
        board.world_state.player_states.four = None;
        board.world_state.player_states.three = None;
        assert_eq!(kickoff_receiver(&board), None);
        assert!(is_own_kickoff(&mut board));
        board.world_state.now = board.world_state.now + Duration::from_secs(2);
        assert!(!is_own_kickoff(&mut board));

        let mut board = blackboard();
        board.world_state.now = board.world_state.now + Duration::from_secs(2);
        assert_eq!(kickoff_receiver(&board), None);
        board.player_number = board.parameters.goalkeeper.player_number;
        assert!(!is_own_kickoff(&mut board));
        board.player_number = PlayerNumber::Two;
        board.world_state.robot.primary_state = PrimaryState::Ready;
        board
            .world_state
            .filtered_game_controller_state
            .as_mut()
            .unwrap()
            .kicking_team = Some(Team::Opponent);
        assert!(board.try_sending_state_message().is_none());
    }

    #[test]
    fn kickoff_pass_rejects_stale_receiver_blocked_lane_and_other_restarts() {
        let mut board = blackboard();
        board.world_state.now = board.world_state.now + Duration::from_millis(500);
        assert!(kickoff_pass_target(&board, PlayerNumber::Five).is_none());
        let mut board = blackboard();
        board
            .world_state
            .obstacles
            .push(Obstacle::robot(point![-0.1, -0.5], 0.2, 0.2));
        assert!(kickoff_pass_target(&board, PlayerNumber::Five).is_none());
        let mut board = blackboard();
        board
            .world_state
            .filtered_game_controller_state
            .as_mut()
            .unwrap()
            .kicking_team = Some(Team::Opponent);
        assert!(kickoff_pass_target(&board, PlayerNumber::Five).is_none());
        let mut board = blackboard();
        board.ball.as_mut().unwrap().position = point![0.4, 0.0];
        assert!(kickoff_pass_target(&board, PlayerNumber::Five).is_none());
        let mut board = blackboard();
        let staged = kickoff_receiver_position(&board);
        board
            .world_state
            .player_states
            .five
            .as_mut()
            .unwrap()
            .inner
            .pose = Pose2::new(point![staged.x() - 0.5, staged.y()], 0.0);
        assert!(kickoff_pass_target(&board, PlayerNumber::Five).is_none());
        let mut board = blackboard();
        let stance = kickoff_receiving_pose(&board).unwrap();
        board
            .world_state
            .player_states
            .five
            .as_mut()
            .unwrap()
            .inner
            .pose = Pose2::new(stance.position(), 0.0);
        assert!(kickoff_pass_target(&board, PlayerNumber::Five).is_none());
    }

    #[test]
    fn blocked_kickoff_pass_holds_instead_of_falling_through_to_goal_shot() {
        let mut board = blackboard();
        board.player_number = PlayerNumber::Two;
        board.world_state.robot.primary_state = PrimaryState::Playing;
        board.world_state.ball = Some(types::world_state::BallState::new_at_center(
            Default::default(),
        ));
        board.parameters.control.is_simple = true;
        board.parameters.kicking.kick_activation_distance = 1.3;
        board.parameters.kicking.strategy.maximum_shot_distance = 6.0;
        board
            .world_state
            .obstacles
            .push(Obstacle::robot(point![-0.1, -0.5], 0.2, 0.2));
        let tree = crate::tree::create_tree();
        assert_eq!(tree.tick_with_trace(&mut board).0, Status::Success);
        assert_eq!(board.body_motion, Some(BodyMotion::Stand));
        assert_eq!(board.ball_action, None);
        assert_eq!(board.kick_target, None);

        board.world_state.obstacles.clear();
        assert_eq!(tree.tick_with_trace(&mut board).0, Status::Success);
        assert_eq!(
            board.ball_action,
            Some(BallAction::Pass {
                receiver: PlayerNumber::Five
            })
        );
        assert!(board.kick_target.unwrap().y() < 0.0);

        board.world_state.now = board.world_state.now + Duration::from_millis(500);
        assert_eq!(tree.tick_with_trace(&mut board).0, Status::Success);
        assert_eq!(board.body_motion, Some(BodyMotion::Stand));
        assert_eq!(board.ball_action, None);
    }

    #[test]
    fn prepared_kickoff_survives_game_controller_playing_and_releases_safely() {
        let mut board = blackboard();
        board.player_number = PlayerNumber::Two;
        board.world_state.robot.primary_state = PrimaryState::Ready;
        board
            .world_state
            .filtered_game_controller_state
            .as_mut()
            .unwrap()
            .game_state = FilteredGameState::Ready;
        board.update_kickoff();
        board.world_state.robot.primary_state = PrimaryState::Playing;
        board
            .world_state
            .filtered_game_controller_state
            .as_mut()
            .unwrap()
            .game_state = FilteredGameState::Playing {
            ball_is_free: true,
            kick_off: false,
        };
        board.update_kickoff();
        assert!(own_kickoff(&board));
        assert!(needs_kickoff_updates(&board));

        let mut moved = board.clone();
        moved.ball.as_mut().unwrap().position = point![0.5, 0.0];
        moved.update_kickoff();
        assert!(!own_kickoff(&moved));
        assert!(moved.kickoff_started.is_none());

        board.world_state.now = board.world_state.now + Duration::from_secs(11);
        board.update_kickoff();
        assert!(!own_kickoff(&board));
        assert!(!needs_kickoff_updates(&board));
    }
}
