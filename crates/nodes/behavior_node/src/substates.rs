use std::time::Duration;

use coordinate_systems::Field;
use hsl_network_messages::{PlayerNumber, SubState, Team};
use linear_algebra::{Point2, Rotation2, point};
use types::{
    behavior_tree::Status, field_dimensions::Side,
    filtered_game_controller_state::FilteredGameControllerState, obstacles::ObstacleKind,
};

use crate::{
    action,
    behavior_tree::Node,
    condition,
    conditions::{hulks_is_kicking_team, is_close_to_ball, is_close_to_ball_aligned},
    kick::{
        execution::{approach_restart_ball, execute_ball_action},
        geometry::pass_openings,
        kick_subtree,
        passing::{RESTART_RECEIVER_EXTRA_DISTANCE, is_pass_receiver},
        reception::receive_rolling_ball,
        strategy::{BallAction, select_restart_ball_action},
    },
    negation,
    node::Blackboard,
    penalty_shootout::penalty_kick_striker_subtree,
    selection, sequence, subtree,
    walk::{walk_to_ball_subtree, walk_to_block_position},
};

pub fn sub_state_subtree() -> Node<Blackboard> {
    selection!(
        sequence!(
            condition!(hulks_is_kicking_team),
            selection!(
                sequence!(
                    condition!(is_sub_state, SubState::PenaltyKick),
                    subtree!(penalty_kick_striker_subtree),
                ),
                sequence!(condition!(is_pass_receiver), action!(receive_rolling_ball)),
                sequence!(
                    action!(select_restart_ball_action),
                    action!(execute_ball_action),
                ),
                action!(wait_for_restart_receiver),
                sequence!(
                    condition!(restart_requires_pass),
                    selection!(
                        action!(dribble_restart),
                        action!(approach_restart_ball),
                        action!(crate::actions::stand)
                    ),
                ),
                sequence!(
                    negation!(condition!(is_close_to_ball_aligned)),
                    subtree!(walk_to_ball_subtree)
                ),
                subtree!(kick_subtree)
            )
        ),
        sequence!(
            selection!(
                sequence!(
                    selection!(
                        condition!(is_sub_state, SubState::ThrowIn),
                        condition!(is_sub_state, SubState::IndirectFreeKick),
                        condition!(is_sub_state, SubState::DirectFreeKick),
                    ),
                    action!(set_block_position_field),
                ),
                sequence!(
                    condition!(is_sub_state, SubState::CornerKick),
                    action!(set_block_position_corner),
                ),
                sequence!(
                    condition!(is_sub_state, SubState::PenaltyKick),
                    action!(set_block_position_penalty_kick),
                ),
                sequence!(
                    condition!(is_sub_state, SubState::GoalKick),
                    action!(set_block_position_goal_kick),
                ),
            ),
            action!(walk_to_block_position)
        )
    )
}

fn restart_players(blackboard: &Blackboard) -> Vec<(PlayerNumber, Point2<Field>)> {
    let world = &blackboard.world_state;
    let (Some(game), Some(transform)) = (
        &world.filtered_game_controller_state,
        world.robot.ground_to_field,
    ) else {
        return Vec::new();
    };
    game.penalties
        .iter()
        .filter_map(|(player, penalty)| {
            if penalty.is_some() || player == blackboard.parameters.goalkeeper.player_number {
                return None;
            }
            let position = if player == blackboard.player_number {
                transform.as_pose().position()
            } else {
                let state = world.player_states[player].as_ref()?;
                if state.time > world.now
                    || world.now.duration_since(state.time)
                        >= blackboard
                            .parameters
                            .kicking
                            .strategy
                            .passing
                            .maximum_receiver_age
                {
                    return None;
                }
                state.inner.pose.position()
            };
            position
                .inner
                .iter()
                .all(|coordinate| coordinate.is_finite())
                .then_some((player, position))
        })
        .collect()
}

pub(crate) fn restart_pass_is_open(blackboard: &Blackboard, receiver: Point2<Field>) -> bool {
    let (Some(ball), Some(transform)) = (
        &blackboard.ball,
        blackboard.world_state.robot.ground_to_field,
    ) else {
        return false;
    };
    if receiver.x().abs() >= blackboard.field_dimensions.length / 2.0
        || receiver.y().abs() >= blackboard.field_dimensions.width / 2.0
    {
        return false;
    }
    let parameters = &blackboard.parameters.kicking.strategy;
    let support_distance = (blackboard.field_dimensions.center_circle_diameter / 2.0).max(
        parameters.passing.receiver_standoff
            + 2.0 * blackboard.parameters.walking.path_planning.robot_radius,
    );
    if blackboard
        .world_state
        .filtered_game_controller_state
        .as_ref()
        .is_some_and(|game| {
            game.kicking_team == Some(Team::Hulks) && game.sub_state == Some(SubState::CornerKick)
        })
        && ball.position.x() > 0.0
        && receiver.x()
            > ball.position.x() - 2.0 * support_distance + parameters.passing.receiver_standoff
    {
        return false;
    }
    if blackboard
        .world_state
        .filtered_game_controller_state
        .as_ref()
        .is_some_and(|game| {
            game.kicking_team == Some(Team::Hulks) && game.sub_state == Some(SubState::ThrowIn)
        })
        && (receiver.x() - ball.position.x()).abs()
            < 1.5 * support_distance - parameters.passing.receiver_standoff
    {
        return false;
    }
    let players = restart_players(blackboard);
    let Some((taker_number, taker)) = players.iter().min_by(|(_, a), (_, b)| {
        (*a - ball.position)
            .norm_squared()
            .total_cmp(&(*b - ball.position).norm_squared())
    }) else {
        return false;
    };
    let radius = blackboard.parameters.walking.path_planning.robot_radius;
    // The launching robot's body is absent from its own obstacle view as well.
    let obstacles: Vec<_> = blackboard
        .world_state
        .obstacles
        .iter()
        .filter(|obstacle| {
            *taker_number == blackboard.player_number
                || !matches!(
                    obstacle.kind,
                    ObstacleKind::Robot | ObstacleKind::Person | ObstacleKind::Unknown
                )
                || (transform * obstacle.position - *taker).norm()
                    > radius + obstacle.radius_at_foot_height
        })
        .cloned()
        .collect();
    let mut passing = parameters.passing.clone();
    passing.minimum_distance += RESTART_RECEIVER_EXTRA_DISTANCE;
    !pass_openings(
        ball.position,
        receiver,
        transform,
        &obstacles,
        &blackboard.field_dimensions,
        &passing,
        parameters.obstacle_clearance,
    )
    .is_empty()
}

pub(crate) fn restart_pass_receiver(
    blackboard: &Blackboard,
) -> Option<(PlayerNumber, Point2<Field>)> {
    let world = &blackboard.world_state;
    let game = world.filtered_game_controller_state.as_ref()?;
    if game.kicking_team != Some(Team::Hulks)
        || !matches!(
            game.sub_state,
            Some(SubState::CornerKick | SubState::ThrowIn)
        )
    {
        return None;
    }
    let ball = blackboard.ball.as_ref()?.position;
    let parameters = &blackboard.parameters.kicking.strategy;
    parameters.validate().ok()?;
    let players = restart_players(blackboard);
    let taker = players
        .iter()
        .min_by(|(_, a), (_, b)| {
            (*a - ball)
                .norm_squared()
                .total_cmp(&(*b - ball).norm_squared())
        })?
        .0;
    let radius = blackboard.parameters.walking.path_planning.robot_radius;
    let distance = (blackboard.field_dimensions.center_circle_diameter / 2.0)
        .max(parameters.passing.receiver_standoff + 2.0 * radius);
    let field = &blackboard.field_dimensions;
    let margin = radius + 0.05;
    if !ball.inner.iter().all(|coordinate| coordinate.is_finite())
        || !margin.is_finite()
        || margin < 0.0
        || !field.length.is_finite()
        || field.length <= 2.0 * margin
        || !field.width.is_finite()
        || field.width <= 2.0 * margin
    {
        return None;
    }
    let x_sign = if game.sub_state == Some(SubState::CornerKick) && ball.x() > 0.0
        || ball.x() + distance >= field.length / 2.0 - margin
    {
        -1.0
    } else {
        1.0
    };
    let y_sign = if ball.y() >= 0.0 { -1.0 } else { 1.0 };
    let backward: f32 = if game.sub_state == Some(SubState::CornerKick) {
        2.0
    } else {
        1.5
    };
    for (forward, inward) in [(backward, 1.0), (backward + 1.0, 1.0), (backward, 2.0)] {
        let distance = distance.max(
            (parameters.passing.minimum_distance
                + RESTART_RECEIVER_EXTRA_DISTANCE
                + parameters.passing.receiver_standoff
                + parameters.passing.target_half_width)
                / forward.hypot(inward),
        );
        let target = point![
            (ball.x() + x_sign * distance * forward)
                .clamp(-field.length / 2.0 + margin, field.length / 2.0 - margin),
            (ball.y() + y_sign * distance * inward)
                .clamp(-field.width / 2.0 + margin, field.width / 2.0 - margin)
        ];
        if !restart_pass_is_open(blackboard, target) {
            continue;
        }
        let receiver = players
            .iter()
            .filter(|(player, _)| *player != taker)
            .min_by(|(_, a), (_, b)| {
                (*a - target)
                    .norm_squared()
                    .total_cmp(&(*b - target).norm_squared())
            })?
            .0;
        return Some((receiver, target));
    }
    None
}

fn wait_for_restart_receiver(blackboard: &mut Blackboard) -> Status {
    if blackboard.ball_action.is_some()
        || blackboard.last_motion_type == Some(types::motion_type::MotionType::Kick)
        || !blackboard.has_team_message_budget()
        || restart_pass_receiver(blackboard).is_none()
        || !is_close_to_ball(blackboard)
    {
        return Status::Failure;
    }
    let now = blackboard.world_state.now;
    let since = *blackboard.restart_pass_wait_started.get_or_insert(now);
    if now.duration_since(since) >= Duration::from_secs(5) {
        return Status::Failure;
    }
    blackboard.ball_action = None;
    blackboard.kick_target = None;
    crate::actions::stand(blackboard)
}

fn restart_requires_pass(blackboard: &mut Blackboard) -> bool {
    is_sub_state(blackboard, SubState::ThrowIn)
        || is_sub_state(blackboard, SubState::IndirectFreeKick)
}

fn dribble_restart(blackboard: &mut Blackboard) -> Status {
    blackboard.ball_action = Some(BallAction::Dribble);
    if crate::kick::execution::resolve_target(blackboard).is_none() {
        blackboard.ball_action = None;
        blackboard.kick_target = None;
        return Status::Failure;
    }
    execute_ball_action(blackboard)
}

pub fn is_in_sub_state(blackboard: &mut Blackboard) -> bool {
    matches!(
        blackboard.world_state.filtered_game_controller_state,
        Some(FilteredGameControllerState {
            sub_state: Some(_),
            kicking_team: Some(_),
            ..
        })
    )
}

pub fn is_sub_state(blackboard: &mut Blackboard, sub_state: SubState) -> bool {
    matches!(
        blackboard.world_state.filtered_game_controller_state,
        Some(FilteredGameControllerState {
            sub_state: Some(current_sub_state),
            ..
        }) if current_sub_state == sub_state
    )
}

pub fn set_block_position_field(blackboard: &mut Blackboard) -> Status {
    if let (Some(ball), Some(ground_to_field)) = (
        &blackboard.ball,
        &blackboard.world_state.robot.ground_to_field,
    ) {
        let ball_in_ground = ground_to_field.inverse() * ball.position;
        let goal_position =
            ground_to_field.inverse() * point!(-blackboard.field_dimensions.length / 2.0, 0.0);
        let direction = (goal_position - ball_in_ground).normalize();

        let distance_to_ball = (blackboard.field_dimensions.center_circle_diameter / 2.0
            + blackboard.parameters.substates.blocking_distance_offset)
            .max(
                blackboard.field_dimensions.center_circle_diameter / 2.0
                    + blackboard.parameters.walking.path_planning.robot_radius,
            );

        blackboard.walk_position = Some(ball_in_ground + (direction * distance_to_ball));

        Status::Success
    } else {
        Status::Failure
    }
}

pub fn set_block_position_corner(blackboard: &mut Blackboard) -> Status {
    if let (Some(ball), Some(ground_to_field)) = (
        &blackboard.ball,
        &blackboard.world_state.robot.ground_to_field,
    ) {
        let ball_position = ground_to_field.inverse() * ball.position;
        let goal_position =
            ground_to_field.inverse() * point!(-blackboard.field_dimensions.length / 2.0, 0.0);
        let angle_direction = match ball.field_side {
            Side::Left => 1.0,
            Side::Right => -1.0,
        };

        let parameters = &blackboard.parameters.substates;

        let direction = Rotation2::new(angle_direction * parameters.corner_kick_blocking_angle)
            * (goal_position - ball_position).normalize();

        let distance_to_ball = (blackboard.field_dimensions.center_circle_diameter / 2.0
            + parameters.blocking_distance_offset)
            .max(
                blackboard.field_dimensions.center_circle_diameter / 2.0
                    + blackboard.parameters.walking.path_planning.robot_radius,
            );

        blackboard.walk_position = Some(ball_position + direction * distance_to_ball);

        Status::Success
    } else {
        Status::Failure
    }
}

pub fn set_block_position_penalty_kick(blackboard: &mut Blackboard) -> Status {
    if let Some(ground_to_field) = &blackboard.world_state.robot.ground_to_field {
        let field_dimensions = blackboard.field_dimensions;
        let penalty_area_x = -field_dimensions.length / 2.0 + field_dimensions.penalty_area_length;
        let distance_to_ball_along_x =
            field_dimensions.penalty_area_length - field_dimensions.penalty_marker_distance;
        let distance_to_ball = (blackboard.field_dimensions.center_circle_diameter / 2.0
            + blackboard.parameters.substates.blocking_distance_offset)
            .max(blackboard.field_dimensions.center_circle_diameter / 2.0);

        let line_position = (distance_to_ball.powi(2) - distance_to_ball_along_x.powi(2))
            .max(0.0)
            .sqrt();

        let side_sign = (ground_to_field * point!(0.0, 0.0)).y().signum();

        blackboard.walk_position =
            Some(ground_to_field.inverse() * point!(penalty_area_x, side_sign * line_position));

        Status::Success
    } else {
        Status::Failure
    }
}

pub fn set_block_position_goal_kick(blackboard: &mut Blackboard) -> Status {
    if let (Some(ball), Some(ground_to_field)) = (
        &blackboard.ball,
        &blackboard.world_state.robot.ground_to_field,
    ) {
        let field_dimensions = blackboard.field_dimensions;
        let ball_position = ball.position;
        let goal_position = point!(-field_dimensions.length / 2.0, 0.0);
        let direction = goal_position - ball_position;
        if direction.x().abs() <= f32::EPSILON {
            return Status::Failure;
        }

        let blocking_x = field_dimensions.length / 2.0
            - field_dimensions.penalty_area_length
            - blackboard.parameters.walking.path_planning.robot_radius;
        let distance_along_line = (blocking_x - ball_position.x()) / direction.x();
        let blocking_position = ball_position + direction * distance_along_line;

        blackboard.walk_position = Some(ground_to_field.inverse() * blocking_position);

        Status::Success
    } else {
        Status::Failure
    }
}

#[cfg(test)]
mod restart_tests {
    use super::*;
    use crate::{
        kick::{passing::prepare_for_restart_pass, strategy::BallAction},
        test_support::{player_state, restart_blackboard},
    };
    use linear_algebra::Pose2;
    use types::{
        motion_command::BodyMotion, motion_type::MotionType, time_wrapper::TimeWrapper,
        world_state::PlayerState,
    };

    fn add_receiver(board: &mut Blackboard, position: Point2<Field>) {
        board.world_state.player_states.two = Some(TimeWrapper {
            time: board.world_state.now,
            inner: PlayerState {
                pose: Pose2::new(position, 0.0),
                ..player_state()
            },
        });
    }

    #[test]
    fn own_restarts_pass_from_touchlines_and_without_goal_alignment() {
        for (state, ball, receiver) in [
            (SubState::GoalKick, point![-3.5, 0.0], point![-1.5, 0.0]),
            (SubState::ThrowIn, point![0.0, 3.0], point![-2.0, 1.0]),
            (SubState::CornerKick, point![4.5, 3.0], point![1.5, 1.0]),
            (
                SubState::IndirectFreeKick,
                point![0.0, 0.0],
                point![2.0, 1.0],
            ),
            (SubState::DirectFreeKick, point![0.0, 0.0], point![2.0, 1.0]),
        ] {
            let mut board = restart_blackboard(state, ball);
            add_receiver(&mut board, receiver);
            let tree = sub_state_subtree();
            assert_eq!(
                tree.tick_with_trace(&mut board).0,
                Status::Success,
                "{state:?}"
            );
            assert_eq!(
                board.ball_action,
                Some(BallAction::Pass {
                    receiver: PlayerNumber::Two
                }),
                "{state:?}"
            );
            assert_eq!(board.body_motion, Some(BodyMotion::Stand));
            assert!(board.try_sending_state_message().is_some());
            board.world_state.now = board.world_state.now + Duration::from_millis(150);
            tree.tick_with_trace(&mut board);
            assert!(
                matches!(
                    board.body_motion,
                    Some(BodyMotion::Kick {
                        soft: false,
                        quick: false,
                        ..
                    })
                ),
                "{state:?}"
            );
        }
    }

    #[test]
    fn throwin_and_indirect_without_a_pass_dribble_instead_of_shooting() {
        for state in [SubState::ThrowIn, SubState::IndirectFreeKick] {
            let mut board = restart_blackboard(state, point![0.0, 0.0]);
            let tree = sub_state_subtree();
            for age in [0, 6] {
                board.world_state.now = board.world_state.now + Duration::from_secs(age);
                tree.tick_with_trace(&mut board);
                assert!(matches!(
                    board.body_motion,
                    Some(BodyMotion::Kick {
                        target_speed: 0.5,
                        quick: false,
                        strong: false,
                        soft: false,
                        ..
                    })
                ));
                assert_eq!(board.ball_action, Some(BallAction::Dribble));
                let target =
                    board.world_state.robot.ground_to_field.unwrap() * board.kick_target.unwrap();
                assert!(target.x().abs() < board.field_dimensions.length / 2.0);
                assert!(target.y().abs() < board.field_dimensions.width / 2.0);
            }
            add_receiver(&mut board, point![2.5, 1.0]);
            tree.tick_with_trace(&mut board);
            assert!(matches!(board.ball_action, Some(BallAction::Pass { .. })));
        }
    }

    #[test]
    fn throwin_and_indirect_hold_when_dribbling_is_disabled_or_blocked() {
        for state in [SubState::ThrowIn, SubState::IndirectFreeKick] {
            for blocked in [false, true] {
                let mut board = restart_blackboard(state, Point2::origin());
                if blocked {
                    board.world_state.obstacles = vec![types::obstacles::Obstacle::robot(
                        point![0.6, 0.0],
                        0.5,
                        0.5,
                    )];
                } else {
                    board.parameters.kicking.strategy.maximum_dribble_distance = 0.0;
                }
                sub_state_subtree().tick_with_trace(&mut board);
                assert_eq!(board.body_motion, Some(BodyMotion::Stand));
                assert!(board.ball_action.is_none());
            }
        }
    }

    #[test]
    fn distant_throwin_and_indirect_takers_approach_observed_ball_before_dribbling_without_a_receiver()
     {
        for state in [SubState::ThrowIn, SubState::IndirectFreeKick] {
            let mut board = restart_blackboard(state, Point2::origin());
            board.world_state.robot.ground_to_field =
                Some(Pose2::new(point![-3.0, 0.0], 0.0).as_transform());
            // A measured velocity changes neither the observed approach point nor its standoff.
            board.ball.as_mut().unwrap().velocity = linear_algebra::vector![-2.0, 3.0];
            let tree = sub_state_subtree();
            assert_eq!(tree.tick_with_trace(&mut board).0, Status::Success);
            let Some(BodyMotion::Walk { ref path, .. }) = board.body_motion else {
                panic!("{state:?}: distant taker must approach");
            };
            let types::path::PathSegment::LineSegment(last) = path.last_segment() else {
                panic!("expected a direct approach");
            };
            let distance_to_ball = (linear_algebra::point![3.0, 0.0] - last.1).norm();
            assert!(last.1.x() > 0.0 && last.1.y().abs() < 1e-5);
            assert!(distance_to_ball + 1e-5 >= board.parameters.kicking.approach_ball_standoff);
            assert!(distance_to_ball <= board.parameters.kicking.kick_activation_distance);
            assert_eq!(board.ball_action, Some(BallAction::Dribble));
            assert!(board.restart_pass_wait_started.is_none());
            let mut locked = board.clone();
            locked.last_motion_type = Some(MotionType::StandUp);
            locked.last_motion_switch_time = locked.world_state.now;
            tree.tick_with_trace(&mut locked);
            assert_eq!(locked.body_motion, Some(BodyMotion::Stand));
            board.world_state.robot.ground_to_field =
                Some(Pose2::new(point![-0.6, 0.0], 0.0).as_transform());
            board.ball.as_mut().unwrap().velocity = linear_algebra::Vector2::zeros();
            tree.tick_with_trace(&mut board);
            assert!(matches!(
                board.body_motion,
                Some(BodyMotion::Kick {
                    target_speed: 0.5,
                    ..
                })
            ));
            assert_eq!(board.ball_action, Some(BallAction::Dribble));
        }
    }

    #[test]
    fn blocked_corridor_reselects_and_invalid_receivers_do_not_enter_restart_execution() {
        let mut board = restart_blackboard(SubState::DirectFreeKick, point![0.0, 0.0]);
        add_receiver(&mut board, point![3.0, 0.0]);
        assert_eq!(select_restart_ball_action(&mut board), Status::Success);
        assert_eq!(execute_ball_action(&mut board), Status::Success);
        board.try_sending_state_message();
        let transform = board.world_state.robot.ground_to_field.unwrap();
        board.world_state.obstacles = vec![types::obstacles::Obstacle::robot(
            transform.inverse() * point![1.0, 0.0],
            0.6,
            0.6,
        )];
        assert_eq!(execute_ball_action(&mut board), Status::Success);
        assert_eq!(board.ball_action, None);
        assert_eq!(board.body_motion, Some(BodyMotion::Stand));
        assert!(board.last_announced_pass.is_none());
        board.world_state.obstacles.clear();
        for case in 0..5 {
            let mut invalid = board.clone();
            let state = invalid.world_state.player_states.two.as_mut().unwrap();
            match case {
                0 => state.time = invalid.world_state.now + Duration::from_secs(1),
                1 => state.time = invalid.world_state.now - Duration::from_secs(3),
                2 => state.inner.pose = point![f32::NAN, 0.0].into(),
                3 => invalid.parameters.goalkeeper.player_number = PlayerNumber::Two,
                _ => {
                    invalid
                        .world_state
                        .filtered_game_controller_state
                        .as_mut()
                        .unwrap()
                        .penalties
                        .two = Some(hsl_network_messages::Penalty::PickUp {
                        remaining: Duration::from_secs(1),
                    })
                }
            }
            assert_eq!(
                select_restart_ball_action(&mut invalid),
                Status::Failure,
                "case {case}"
            );
        }
    }

    #[test]
    fn restart_boundary_roundoff_is_accepted_but_real_outside_ball_stops() {
        for (outside, allowed) in [(1e-5, true), (0.01, false)] {
            let mut board = restart_blackboard(SubState::ThrowIn, point![0.0, 3.0 + outside]);
            add_receiver(&mut board, point![2.5, 1.0]);
            assert_eq!(
                select_restart_ball_action(&mut board) == Status::Success,
                allowed
            );
        }
    }

    #[test]
    fn corner_and_throwin_stage_one_receiver_with_observed_positions() {
        for state in [SubState::CornerKick, SubState::ThrowIn] {
            let ball = if state == SubState::CornerKick {
                point![7.0, 4.5]
            } else {
                point![-1.5, 4.5]
            };
            let mut taker = restart_blackboard(state, ball);
            taker.field_dimensions.length = 14.0;
            taker.field_dimensions.width = 9.0;
            taker.field_dimensions.center_circle_diameter = 3.0;
            taker.parameters.kicking.strategy.passing.minimum_distance = 4.0;
            add_receiver(&mut taker, point![-3.0, -0.5]);
            taker.world_state.player_states.four = Some(TimeWrapper {
                time: taker.world_state.now,
                inner: PlayerState {
                    pose: point![-4.0, -4.0].into(),
                    ..player_state()
                },
            });
            let (selected, target) = restart_pass_receiver(&taker).expect("a legal staging lane");
            assert!(target.x().abs() + 0.4 < 7.0 && target.y().abs() + 0.4 < 4.5);
            assert!(restart_pass_is_open(&taker, target));
            assert_eq!(select_restart_ball_action(&mut taker), Status::Failure);
            assert_eq!(wait_for_restart_receiver(&mut taker), Status::Success);
            assert!(taker.restart_pass_wait_started.is_some());
            let mut staged = 0;
            for number in [PlayerNumber::Two, PlayerNumber::Four] {
                let mut supporter = taker.clone();
                supporter.player_number = number;
                supporter.world_state.player_states.three = Some(TimeWrapper {
                    time: taker.world_state.now,
                    inner: PlayerState {
                        pose: taker.world_state.robot.ground_to_field.unwrap().as_pose(),
                        ..player_state()
                    },
                });
                let position = supporter.world_state.player_states[number]
                    .as_ref()
                    .unwrap()
                    .inner
                    .pose
                    .position();
                supporter.world_state.robot.ground_to_field =
                    Some(Pose2::new(position, 0.0).as_transform());
                if prepare_for_restart_pass(&mut supporter) == Status::Success {
                    staged += 1;
                    assert_eq!(number, selected);
                }
            }
            assert_eq!(staged, 1, "{state:?}");
            taker.world_state.player_states[selected]
                .as_mut()
                .unwrap()
                .inner
                .pose = Pose2::new(target, 0.0);
            assert_eq!(select_restart_ball_action(&mut taker), Status::Success);
            let mut invalid_field = taker.clone();
            invalid_field.field_dimensions.width = 0.0;
            assert!(restart_pass_receiver(&invalid_field).is_none());
            taker
                .world_state
                .filtered_game_controller_state
                .as_mut()
                .unwrap()
                .sub_state = None;
            taker.update_restart_pass();
            assert!(taker.restart_pass_wait_started.is_none());
            // An unannounced plan cannot survive restart release.
            taker.last_motion_type = Some(MotionType::Walk);
            assert!(crate::kick::execution::resolve_target(&taker).is_none());
            assert_eq!(select_restart_ball_action(&mut taker), Status::Failure);
        }
    }
}
