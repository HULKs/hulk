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
    execution::execute_ball_action, passing::RESTART_RECEIVER_EXTRA_DISTANCE, strategy::BallAction,
};

pub fn kickoff_kicker(blackboard: &Blackboard) -> Option<PlayerNumber> {
    blackboard
        .world_state
        .filtered_game_controller_state
        .as_ref()?
        .penalties
        .iter()
        .filter(|(player, penalty)| {
            *player != blackboard.parameters.goalkeeper.player_number && penalty.is_none()
        })
        .map(|(player, _)| player)
        .max()
}

pub fn kickoff_receiver(blackboard: &Blackboard) -> Option<PlayerNumber> {
    let kicker = kickoff_kicker(blackboard)?;
    blackboard
        .world_state
        .filtered_game_controller_state
        .as_ref()?
        .penalties
        .iter()
        .filter(|(player, penalty)| {
            *player != kicker
                && *player != blackboard.parameters.goalkeeper.player_number
                && penalty.is_none()
        })
        .map(|(player, _)| player)
        .filter(|&player| player_pose(blackboard, player, Duration::from_secs(2)).is_some())
        .max()
}

pub fn kickoff_receiver_position(blackboard: &Blackboard) -> Point2<Field> {
    let radius = blackboard.parameters.walking.path_planning.robot_radius;
    let y = (blackboard.parameters.kickoff.relative_aggressive_positions[0]
        .y()
        .abs()
        * blackboard.field_dimensions.width
        / 2.0)
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
    let field = &blackboard.field_dimensions;
    let clearance = blackboard.parameters.walking.path_planning.robot_radius + 0.05;
    if !clearance.is_finite()
        || clearance < 0.0
        || !field.length.is_finite()
        || field.length <= 2.0 * clearance
        || !field.width.is_finite()
        || field.width <= 2.0 * clearance
    {
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
        && Some(blackboard.parameters.goalkeeper.player_number) != kickoff_kicker(blackboard)
        && player_pose(
            blackboard,
            kickoff_kicker(blackboard).unwrap_or(blackboard.player_number),
            Duration::from_secs(2),
        )
        .is_some();
    if !active || receiver.is_some() {
        blackboard.kickoff_receiver_wait_started = None;
        return active;
    }
    if Some(blackboard.player_number) != kickoff_kicker(blackboard) {
        return false;
    }
    let now = blackboard.world_state.now;
    let since = *blackboard.kickoff_receiver_wait_started.get_or_insert(now);
    now.duration_since(since) < Duration::from_secs(2)
}

pub(crate) fn needs_kickoff_updates(blackboard: &Blackboard) -> bool {
    kickoff_passing_enabled(blackboard)
        && blackboard.player_number != blackboard.parameters.goalkeeper.player_number
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
    kickoff_passing_enabled(blackboard)
        && blackboard
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
            .map(|transform| transform.as_pose())
            .filter(|pose| {
                pose.position().inner.iter().all(|v| v.is_finite()) && pose.angle().is_finite()
            });
    }
    let state = world.player_states[player].as_ref()?;
    (state.inner.is_finite()
        && state.time <= world.now
        && world.now.duration_since(state.time) < maximum_age)
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
    Some(blackboard.player_number) == kickoff_kicker(blackboard)
        && blackboard.has_team_message_budget()
}

fn is_other_field_player(blackboard: &mut Blackboard) -> bool {
    let player = blackboard.player_number;
    Some(player) != kickoff_kicker(blackboard)
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

/// Override the base kickoff slots only for the selected own kicker and receiver.
pub fn kickoff_pose(blackboard: &Blackboard, player: PlayerNumber) -> Option<Pose2<Field>> {
    if !kickoff_passing_enabled(blackboard) {
        return None;
    }
    let field = &blackboard.field_dimensions;
    let clearance = blackboard.parameters.walking.path_planning.robot_radius + 0.05;
    if !clearance.is_finite()
        || clearance < 0.0
        || !field.length.is_finite()
        || field.length <= 2.0 * clearance
        || !field.width.is_finite()
        || field.width <= 2.0 * clearance
    {
        return None;
    }
    let game = blackboard
        .world_state
        .filtered_game_controller_state
        .as_ref()?;
    if game.kicking_team != Some(Team::Hulks)
        || game.sub_state.is_some()
        || !matches!(
            game.game_state,
            FilteredGameState::Ready | FilteredGameState::Set | FilteredGameState::Playing { .. }
        )
    {
        return None;
    }
    if Some(player) == kickoff_receiver(blackboard) {
        if matches!(game.game_state, FilteredGameState::Playing { .. }) {
            return kickoff_receiving_pose(blackboard);
        }
        let position = kickoff_receiver_position(blackboard);
        return Some(Pose2::from_parts(
            position,
            Orientation2::from_vector(-position.coords()),
        ));
    }
    if Some(player) == kickoff_kicker(blackboard) {
        let relative = blackboard
            .parameters
            .kickoff
            .relative_striker_side_kick_position;
        let clearance = blackboard.parameters.walking.path_planning.robot_radius + 0.05;
        let position = point![
            (relative.x() * blackboard.field_dimensions.length / 2.0).min(-clearance),
            relative.y() * blackboard.field_dimensions.width / 2.0
        ];
        return Some(Pose2::from_parts(
            position,
            Orientation2::from_vector(-position.coords()),
        ));
    }
    None
}

fn kickoff_passing_enabled(blackboard: &Blackboard) -> bool {
    blackboard
        .parameters
        .kicking
        .strategy
        .passing
        .maximum_distance
        > 0.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use hsl_network_messages::{HulkMessage, SubState};
    use linear_algebra::vector;
    use types::{
        messages::OutgoingMessage, motion_command::BodyMotion, obstacles::Obstacle,
        primary_state::PrimaryState, time_wrapper::TimeWrapper, world_state::PlayerState,
    };

    fn kickoff() -> Blackboard {
        let mut board =
            crate::test_support::restart_blackboard(SubState::DirectFreeKick, Point2::origin());
        board.player_number = PlayerNumber::Five;
        board
            .world_state
            .filtered_game_controller_state
            .as_mut()
            .unwrap()
            .sub_state = None;
        board
            .world_state
            .filtered_game_controller_state
            .as_mut()
            .unwrap()
            .game_state = FilteredGameState::Playing {
            ball_is_free: false,
            kick_off: true,
        };
        let pose = kickoff_receiving_pose(&board).unwrap();
        board.world_state.player_states.four = Some(TimeWrapper {
            time: board.world_state.now,
            inner: PlayerState {
                pose,
                ..crate::test_support::player_state()
            },
        });
        board
    }

    #[test]
    fn disabled_passing_keeps_base_kickoff_behavior_and_pose_slots() {
        let mut board = kickoff();
        board.parameters.kicking.strategy.passing.maximum_distance = 0.0;
        board.parameters.control.is_simple = true;
        assert!(!own_kickoff(&board));
        assert!(!needs_kickoff_updates(&board));
        assert!(kickoff_pose(&board, board.player_number).is_none());
        assert_eq!(
            kickoff_subtree().tick_with_trace(&mut board).0,
            Status::Failure
        );
        crate::tree::create_tree().tick_with_trace(&mut board);
        assert!(matches!(board.body_motion, Some(BodyMotion::Kick { .. })));
        assert!(!matches!(board.ball_action, Some(BallAction::Pass { .. })));

        board
            .world_state
            .filtered_game_controller_state
            .as_mut()
            .unwrap()
            .game_state = FilteredGameState::Ready;
        assert!(!needs_kickoff_updates(&board));
        assert!(kickoff_pose(&board, board.player_number).is_none());
    }

    #[test]
    fn kickoff_waits_announces_immediately_prepares_150ms_and_forces_strong_when_allowed() {
        let mut board = kickoff();
        board.last_sent_hsl_message_time = Some(board.world_state.now);
        assert_eq!(kickoff_kicker(&board), Some(PlayerNumber::Five));
        assert_eq!(kickoff_receiver(&board), Some(PlayerNumber::Four));
        assert_eq!(select_kickoff_pass(&mut board), Status::Success);
        assert_eq!(execute_ball_action(&mut board), Status::Success);
        assert_eq!(board.body_motion, Some(BodyMotion::Stand));
        let Some(OutgoingMessage::Hsl(HulkMessage::State(message))) =
            board.try_sending_state_message()
        else {
            panic!("pass announcement must bypass adaptive cooldown");
        };
        assert_eq!(message.pass.unwrap().receiver, PlayerNumber::Four);
        let announced_at = board.last_announced_pass.as_ref().unwrap().time;
        for (age, ready) in [(149, false), (150, true)] {
            board.world_state.now = announced_at + Duration::from_millis(age);
            execute_ball_action(&mut board);
            assert_eq!(
                matches!(
                    board.body_motion,
                    Some(BodyMotion::Kick {
                        strong: true,
                        soft: false,
                        quick: false,
                        ..
                    })
                ),
                ready
            );
        }
        board.parameters.kicking.allow_strong_kicks = false;
        execute_ball_action(&mut board);
        assert!(matches!(
            board.body_motion,
            Some(BodyMotion::Kick { strong: false, .. })
        ));
    }

    #[test]
    fn kickoff_side_and_ready_pose_are_legal_and_opponent_positions_keep_base_fallback() {
        for left in [false, true] {
            let mut board = kickoff();
            board.parameters.kickoff.pass_to_left = left;
            let receiving = kickoff_receiving_pose(&board).unwrap();
            board
                .world_state
                .player_states
                .four
                .as_mut()
                .unwrap()
                .inner
                .pose = receiving;
            let position = kickoff_receiver_position(&board);
            assert_eq!(position.y() > 0.0, left);
            assert!(position.x() <= -board.parameters.walking.path_planning.robot_radius);
            assert!(position.coords().norm() > board.field_dimensions.center_circle_diameter / 2.0);
            let target = kickoff_pass_target(&board, PlayerNumber::Four).unwrap();
            assert!(target.x() < 0.0);
            assert!(target.y().abs() < board.field_dimensions.width / 2.0);
            let game = board
                .world_state
                .filtered_game_controller_state
                .as_mut()
                .unwrap();
            game.game_state = FilteredGameState::Ready;
            assert!(kickoff_pose(&board, PlayerNumber::Five).is_some());
            assert!(kickoff_pose(&board, PlayerNumber::Four).is_some());
            board
                .world_state
                .filtered_game_controller_state
                .as_mut()
                .unwrap()
                .kicking_team = Some(Team::Opponent);
            assert!(kickoff_pose(&board, PlayerNumber::Five).is_none());
        }
    }

    #[test]
    fn kickoff_rejects_missing_stale_ineligible_or_blocked_receiver_and_holds() {
        let board = kickoff();
        for case in 0..7 {
            let mut invalid = board.clone();
            match case {
                0 => invalid.world_state.player_states.four = None,
                1 => {
                    invalid
                        .world_state
                        .player_states
                        .four
                        .as_mut()
                        .unwrap()
                        .time = invalid.world_state.now - Duration::from_secs(3)
                }
                2 => {
                    invalid
                        .world_state
                        .player_states
                        .four
                        .as_mut()
                        .unwrap()
                        .time = invalid.world_state.now + Duration::from_secs(1)
                }
                3 => {
                    invalid
                        .world_state
                        .filtered_game_controller_state
                        .as_mut()
                        .unwrap()
                        .penalties
                        .four = Some(hsl_network_messages::Penalty::PickUp {
                        remaining: Duration::from_secs(1),
                    })
                }
                5 => invalid.field_dimensions.length = f32::NAN,
                6 => invalid.field_dimensions.width = 0.0,
                _ => {
                    let target = kickoff_pass_target(&invalid, PlayerNumber::Four).unwrap();
                    let transform = invalid.world_state.robot.ground_to_field.unwrap();
                    invalid.world_state.obstacles.push(Obstacle::robot(
                        transform.inverse() * (target / 2.0),
                        0.6,
                        0.6,
                    ));
                }
            }
            assert!(
                kickoff_pass_target(&invalid, PlayerNumber::Four).is_none(),
                "case {case}"
            );
            assert_eq!(
                kickoff_subtree().tick_with_trace(&mut invalid).0,
                Status::Success
            );
            assert_eq!(invalid.body_motion, Some(BodyMotion::Stand));
            assert!(invalid.ball_action.is_none());
        }
        let mut absent = board;
        absent.world_state.player_states.four = None;
        assert!(is_own_kickoff(&mut absent));
        absent.world_state.now = absent.world_state.now + Duration::from_secs(2);
        assert!(!is_own_kickoff(&mut absent));
    }

    #[test]
    fn ready_set_refresh_poses_and_kickoff_lifecycle_survives_controller_release() {
        let mut board = kickoff();
        for state in [FilteredGameState::Ready, FilteredGameState::Set] {
            board
                .world_state
                .filtered_game_controller_state
                .as_mut()
                .unwrap()
                .game_state = state;
            board.world_state.robot.primary_state = if state == FilteredGameState::Ready {
                PrimaryState::Ready
            } else {
                PrimaryState::Set
            };
            board.world_state.now = board.world_state.now + Duration::from_secs(2);
            board.update_kickoff();
            assert!(board.kickoff_prepared);
            assert!(board.try_sending_state_message().is_some());
        }
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
        assert!(board.kickoff_started.is_some());
        assert!(own_kickoff(&board));
        board.ball.as_mut().unwrap().position += vector![0.6, 0.0];
        board.update_kickoff();
        assert!(board.kickoff_started.is_none());
        assert!(!own_kickoff(&board));
    }
}
