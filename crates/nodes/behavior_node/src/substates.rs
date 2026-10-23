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
    conditions::{hulks_is_kicking_team, is_close_to_ball},
    kick::{
        RESTART_RECEIVER_EXTRA_DISTANCE,
        execution::execute_ball_action,
        geometry::pass_openings,
        strategy::{BallAction, select_ball_action},
    },
    node::Blackboard,
    penalty_shootout::penalty_kick_striker_subtree,
    selection, sequence, subtree,
    walk::walk_to_block_position,
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
                action!(crate::kick::reception::receive_rolling_ball),
                sequence!(
                    action!(select_ball_action),
                    selection!(
                        action!(wait_for_restart_receiver),
                        action!(execute_ball_action)
                    )
                ),
                action!(crate::actions::stand),
            )
        ),
        sequence!(
            selection!(
                sequence!(
                    selection!(
                        condition!(is_sub_state, SubState::ThrowIn),
                        condition!(is_sub_state, SubState::IndirectFreeKick),
                        condition!(is_sub_state, SubState::DirectFreeKick),
                        condition!(is_sub_state, SubState::GoalKick),
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
                )
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
            if penalty.is_some()
                || player == blackboard.parameters.goalkeeper.player_number
                || Some(player) == blackboard.replacement_goalkeeper_coordinator.keeper()
            {
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

pub fn is_opponent_restart_striker(blackboard: &mut Blackboard) -> bool {
    let Some(ball) = blackboard.ball.as_ref() else {
        return false;
    };
    // Rule exclusion zones can cover the ball; elect a fielder independently of ownership there.
    let striker = restart_players(blackboard)
        .into_iter()
        .min_by(|(a_number, a), (b_number, b)| {
            (a - ball.position)
                .norm_squared()
                .total_cmp(&(b - ball.position).norm_squared())
                .then_with(|| a_number.cmp(b_number))
        })
        .map(|(number, _)| number);
    blackboard.last_closest_to_ball = striker == Some(blackboard.player_number);
    blackboard.last_closest_to_ball
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
    if blackboard.ball_action != Some(BallAction::Dribble)
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

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use hsl_network_messages::{PlayerNumber, Team};
    use linear_algebra::{Point2, Pose2};
    use types::{
        behavior_tree::NodeTrace,
        fall_detection::{FallDetection, Posture},
        field_dimensions::FieldDimensions,
        filtered_game_state::FilteredGameState,
        messages::OutgoingMessage,
        motion_command::{BodyMotion, MotionCommand},
        motion_type::MotionType,
        obstacles::Obstacle,
        parameters::{KickStrategyParameters, PassingParameters},
        path::traits::EndPoints,
        primary_state::PrimaryState,
        time_wrapper::TimeWrapper,
        world_state::PlayerState,
    };

    use crate::{kick::strategy::BallAction, node::LastBall};

    use super::*;

    fn succeeded(trace: &NodeTrace, name: &str) -> bool {
        (trace.name == name && trace.status == Status::Success)
            || trace.children.iter().any(|child| succeeded(child, name))
    }

    fn restart(state: SubState, ball: Point2<coordinate_systems::Field>) -> Blackboard {
        let mut board = Blackboard {
            field_dimensions: FieldDimensions::SPL_2025,
            last_motion_type: Some(MotionType::Walk),
            ball: Some(LastBall {
                position: ball,
                velocity: Default::default(),
                age: Default::default(),
                field_side: Side::Left,
                source: Default::default(),
            }),
            ..Default::default()
        };
        board.world_state.now = board.world_state.now + Duration::from_secs(1);
        board.world_state.fall_detection = Some(FallDetection {
            time: board.world_state.now,
            valid_until: board.world_state.now + Duration::from_secs(10),
            posture: Posture::Upright,
        });
        board.world_state.robot.primary_state = PrimaryState::Playing;
        board.world_state.robot.ground_to_field =
            Some(Pose2::new(ball - linear_algebra::vector![0.6, 0.0], 0.7).as_transform());
        board.world_state.filtered_game_controller_state = Some(FilteredGameControllerState {
            kicking_team: Some(Team::Hulks),
            sub_state: Some(state),
            game_state: FilteredGameState::Playing {
                ball_is_free: false,
                kick_off: false,
            },
            remaining_number_of_messages: 1000,
            remaining_time_in_half: Duration::from_secs(100),
            ..crate::test_support::filtered_game_controller_state()
        });
        board.parameters.goalkeeper.player_number = PlayerNumber::One;
        board.parameters.voronoi.grid_resolution = 0.25;
        board.parameters.voronoi.padding = 0.5;
        board
            .parameters
            .walking
            .walk_and_stand
            .orientation_tolerance = 0.05;
        board.parameters.kicking.kick_activation_distance = 1.3;
        board.parameters.kicking.allow_strong_kicks = true;
        board.parameters.kicking.strong_kick_min_target_distance = 3.0;
        board.parameters.kicking.strategy = KickStrategyParameters {
            maximum_shot_distance: 12.0,
            maximum_dribble_distance: 3.0,
            obstacle_clearance: 0.05,
            minimum_shot_angle: 0.02,
            passing: PassingParameters {
                maximum_distance: 8.0,
                minimum_progress: 0.25,
                receiver_standoff: 0.5,
                target_half_width: 0.2,
                maximum_receiver_age: Duration::from_secs(3),
                ..Default::default()
            },
            ..Default::default()
        };
        board
    }

    #[test]
    fn own_restarts_pass_from_touchlines_and_without_goal_alignment() {
        for (state, ball, receiver) in [
            (SubState::GoalKick, point![-3.5, 0.0], point![-1.5, 0.0]),
            (SubState::ThrowIn, point![0.0, 3.0], point![-1.0, 1.0]),
            (SubState::CornerKick, point![4.5, 3.0], point![2.5, 1.0]),
            (
                SubState::IndirectFreeKick,
                point![0.0, 0.0],
                point![2.0, 1.0],
            ),
        ] {
            let mut board = restart(state, ball);
            board.world_state.player_states.two = Some(TimeWrapper {
                time: board.world_state.now,
                inner: PlayerState {
                    pose: Pose2::new(receiver, 0.0),
                    ..crate::test_support::player_state()
                },
            });
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
                        strong: false,
                        ..
                    })
                ),
                "{state:?}"
            );
        }
    }

    #[test]
    fn keeper_goal_kicks_select_a_pass_or_deliberate_shot_before_emergency_clearance() {
        for receiver in [None, Some(PlayerNumber::Two)] {
            let mut board = restart(SubState::GoalKick, point![-3.5, 0.0]);
            board.player_number = PlayerNumber::One;
            board
                .parameters
                .goalkeeper
                .kick_away_ball_maximum_robot_distance = 1.5;
            board.parameters.kicking.strategy.pressure_distance = 1.0;
            board.parameters.kicking.strategy.clear_goal_distance = 5.0;
            let transform = board.world_state.robot.ground_to_field.unwrap();
            board.world_state.obstacles = vec![Obstacle::robot(
                transform.inverse() * point![-3.5, 0.6],
                0.2,
                0.2,
            )];
            if let Some(receiver) = receiver {
                board.world_state.player_states[receiver] = Some(TimeWrapper {
                    time: board.world_state.now,
                    inner: PlayerState {
                        pose: point![-1.5, 0.0].into(),
                        ..crate::test_support::player_state()
                    },
                });
            }
            let tree = crate::goalkeeper::goalkeeper_subtree();
            let (_, trace) = tree.tick_with_trace(&mut board);
            assert!(!succeeded(&trace, "is_goalkeeper_kick_away_needed"));
            if let Some(receiver) = receiver {
                assert_eq!(board.ball_action, Some(BallAction::Pass { receiver }));
                assert_eq!(board.body_motion, Some(BodyMotion::Stand));
                assert!(board.try_sending_state_message().is_some());
                board.world_state.now = board.world_state.now + Duration::from_millis(150);
                tree.tick_with_trace(&mut board);
            } else {
                assert!(matches!(board.ball_action, Some(BallAction::Shoot { .. })));
            }
            assert!(matches!(
                board.body_motion,
                Some(BodyMotion::Kick { quick: false, .. })
            ));
        }
    }

    #[test]
    fn own_restart_without_receiver_shoots_or_plays_infield() {
        let mut board = restart(SubState::GoalKick, point![-3.5, 0.0]);
        sub_state_subtree().tick_with_trace(&mut board);
        assert!(matches!(board.ball_action, Some(BallAction::Shoot { .. })));
        assert!(matches!(
            board.body_motion,
            Some(BodyMotion::Kick {
                strong: true,
                soft: false,
                ..
            })
        ));

        for state in [SubState::ThrowIn, SubState::IndirectFreeKick] {
            let mut board = restart(state, point![0.0, 3.0]);
            sub_state_subtree().tick_with_trace(&mut board);
            assert_eq!(board.ball_action, Some(BallAction::Dribble), "{state:?}");
            assert!(
                matches!(
                    board.body_motion,
                    Some(BodyMotion::Kick {
                        strong: false,
                        soft: false,
                        ..
                    })
                ),
                "{state:?}"
            );
        }
    }

    #[test]
    fn addressed_restart_receiver_holds_position_until_taker_kicks() {
        for (state, ball, position) in [
            (SubState::CornerKick, point![4.5, 3.0], point![2.5, 1.0]),
            (SubState::ThrowIn, point![0.0, 3.0], point![-1.0, 1.0]),
        ] {
            let mut taker = restart(state, ball);
            taker.player_number = PlayerNumber::Two;
            let pose = Pose2::new(
                position,
                (ball.y() - position.y()).atan2(ball.x() - position.x()),
            );
            taker.world_state.player_states.three = Some(TimeWrapper {
                time: taker.world_state.now,
                inner: PlayerState {
                    pose,
                    ..crate::test_support::player_state()
                },
            });
            let tree = crate::tree::create_tree();
            tree.tick_with_trace(&mut taker);
            assert_eq!(
                taker.ball_action,
                Some(BallAction::Pass {
                    receiver: PlayerNumber::Three
                }),
                "{state:?}"
            );
            assert_eq!(taker.body_motion, Some(BodyMotion::Stand));
            let Some(OutgoingMessage::Hsl(hsl_network_messages::HulkMessage::State(message))) =
                taker.try_sending_state_message()
            else {
                panic!("restart pass must be announced");
            };

            let mut receiver = taker.clone();
            receiver.player_number = PlayerNumber::Three;
            receiver.world_state.robot.ground_to_field = Some(pose.as_transform());
            receiver.world_state.player_states = Default::default();
            receiver.world_state.player_states.two = Some(TimeWrapper {
                time: receiver.world_state.now,
                inner: PlayerState {
                    pose: message.pose,
                    pass: message.pass,
                    ..crate::test_support::player_state()
                },
            });
            receiver.ball_action = None;
            receiver.kick_target = None;
            receiver.body_motion = None;
            tree.tick_with_trace(&mut receiver);
            assert!(
                matches!(receiver.body_motion,
                    Some(BodyMotion::WalkWithVelocity { velocity, angular_velocity })
                    if velocity == linear_algebra::Vector2::zeros() && angular_velocity == 0.0),
                "{state:?}"
            );

            taker.world_state.now = taker.world_state.now + Duration::from_millis(150);
            let state = taker.world_state.player_states.three.as_mut().unwrap();
            state.time = taker.world_state.now;
            state.inner.pose = receiver
                .world_state
                .robot
                .ground_to_field
                .unwrap()
                .as_pose();
            taker.kick_target = None;
            taker.body_motion = None;
            tree.tick_with_trace(&mut taker);
            assert!(matches!(
                taker.body_motion,
                Some(BodyMotion::Kick {
                    strong: false,
                    soft: false,
                    ..
                })
            ));

            let transform = taker.world_state.robot.ground_to_field.unwrap();
            let target = transform * taker.kick_target.unwrap();
            taker.last_motion_command = Some(MotionCommand::from_partial_motions(
                taker.body_motion.clone().unwrap(),
                taker.head_motion.unwrap(),
            ));
            taker.last_motion_type = Some(MotionType::Kick);
            taker.last_motion_switch_time = taker.world_state.now;
            // GameController releases the restart on the first small ball movement.
            taker
                .world_state
                .filtered_game_controller_state
                .as_mut()
                .unwrap()
                .sub_state = None;
            taker.ball.as_mut().unwrap().position += (target - ball).normalize() * 0.1;
            taker.kick_target = None;
            taker.body_motion = None;
            tree.tick_with_trace(&mut taker);
            assert_eq!(
                taker.ball_action,
                Some(BallAction::Pass {
                    receiver: PlayerNumber::Three
                })
            );
            assert!(matches!(
                taker.body_motion,
                Some(BodyMotion::Kick {
                    strong: false,
                    soft: false,
                    ..
                })
            ));

            let mut blocked = taker.clone();
            let ball = blocked.ball.as_ref().unwrap().position;
            blocked.world_state.obstacles = vec![Obstacle::robot(
                transform.inverse() * (ball + (target - ball) * 0.5),
                0.4,
                0.4,
            )];
            select_ball_action(&mut blocked);
            assert!(!matches!(
                blocked.ball_action,
                Some(BallAction::Pass { .. })
            ));

            let mut finished = taker.clone();
            finished.last_motion_type = Some(MotionType::Walk);
            select_ball_action(&mut finished);
            assert!(!matches!(
                finished.ball_action,
                Some(BallAction::Pass { .. })
            ));

            taker.world_state.now = taker.last_sent_hsl_message_time.unwrap()
                + hsl_network_messages::PassIntent::TIMEOUT;
            taker.world_state.player_states.three.as_mut().unwrap().time = taker.world_state.now;
            select_ball_action(&mut taker);
            assert!(!matches!(taker.ball_action, Some(BallAction::Pass { .. })));
        }
    }

    #[test]
    fn restart_supporter_moves_until_a_pass_announcement() {
        for (state, ball, position) in [
            (SubState::ThrowIn, point![0.0, 3.0], point![-1.0, 1.0]),
            (SubState::GoalKick, point![-3.5, 0.0], point![-1.5, 1.0]),
        ] {
            let mut receiver = restart(state, ball);
            let taker_pose = receiver
                .world_state
                .robot
                .ground_to_field
                .unwrap()
                .as_pose();
            let pose = Pose2::new(
                position,
                (ball.y() - position.y()).atan2(ball.x() - position.x()),
            );
            receiver.world_state.robot.ground_to_field = Some(pose.as_transform());
            receiver.world_state.player_states.two = Some(TimeWrapper {
                time: receiver.world_state.now,
                inner: PlayerState {
                    pose: taker_pose,
                    ..crate::test_support::player_state()
                },
            });
            let tree = crate::tree::create_tree();
            assert!(!crate::kick::passing::is_pass_receiver(&mut receiver));
            let (_, trace) = tree.tick_with_trace(&mut receiver);
            assert!(succeeded(&trace, "walk_to_voronoi_position"), "{state:?}");
            assert!(matches!(
                receiver.body_motion,
                Some(BodyMotion::Walk { .. })
            ));

            for case in 0..3 {
                let mut moving = receiver.clone();
                match case {
                    0 => moving.parameters.kicking.strategy.passing.maximum_distance = 0.5,
                    1 => {
                        moving
                            .world_state
                            .filtered_game_controller_state
                            .as_mut()
                            .unwrap()
                            .kicking_team = Some(Team::Opponent)
                    }
                    _ => {
                        moving
                            .world_state
                            .filtered_game_controller_state
                            .as_mut()
                            .unwrap()
                            .sub_state = Some(SubState::PenaltyKick)
                    }
                }
                moving.body_motion = None;
                tree.tick_with_trace(&mut moving);
                assert!(
                    matches!(moving.body_motion, Some(BodyMotion::Walk { .. })),
                    "{state:?}, case {case}"
                );
            }

            let mut taker = restart(state, ball);
            taker.player_number = PlayerNumber::Two;
            taker.world_state.player_states.three = Some(TimeWrapper {
                time: receiver.world_state.now,
                inner: PlayerState {
                    pose,
                    ..crate::test_support::player_state()
                },
            });
            // The receiver's older report agrees with its current obstacle position.
            taker.world_state.now = taker.world_state.now + Duration::from_millis(300);
            taker.world_state.obstacles = vec![Obstacle::robot(
                taker_pose.as_transform().inverse() * position,
                0.2,
                0.2,
            )];
            tree.tick_with_trace(&mut taker);
            assert_eq!(
                taker.ball_action,
                Some(BallAction::Pass {
                    receiver: PlayerNumber::Three
                })
            );
            assert!(taker.try_sending_state_message().is_some());
            taker.world_state.now = taker.world_state.now + Duration::from_millis(150);
            taker.kick_target = None;
            taker.body_motion = None;
            tree.tick_with_trace(&mut taker);
            assert!(matches!(
                taker.body_motion,
                Some(BodyMotion::Kick {
                    strong: false,
                    soft: false,
                    ..
                })
            ));
        }
    }

    #[test]
    fn restart_boundary_roundoff_is_accepted_but_real_outside_ball_stops() {
        let mut board = restart(SubState::CornerKick, point![7.000_000_5, 4.500_000_5]);
        board.field_dimensions.length = 14.0;
        board.field_dimensions.width = 9.0;
        board.world_state.player_states.two = Some(TimeWrapper {
            time: board.world_state.now,
            inner: PlayerState {
                pose: Pose2::new(point![4.0, 2.0], 0.0),
                ..crate::test_support::player_state()
            },
        });
        assert_eq!(select_ball_action(&mut board), Status::Success);
        execute_ball_action(&mut board);
        assert_eq!(
            board.ball_action,
            Some(BallAction::Pass {
                receiver: PlayerNumber::Two
            })
        );
        assert!(board.try_sending_state_message().is_some());
        board.world_state.now = board.world_state.now + Duration::from_millis(150);
        execute_ball_action(&mut board);
        assert!(matches!(
            board.body_motion,
            Some(BodyMotion::Kick {
                soft: false,
                strong: false,
                ..
            })
        ));

        for ball in [
            point![7.001, 4.5],
            point![7.0, 4.501],
            point![-7.001, 0.0],
            point![0.0, -4.501],
        ] {
            board.ball.as_mut().unwrap().position = ball;
            assert_eq!(select_ball_action(&mut board), Status::Failure);
            execute_ball_action(&mut board);
            assert_eq!(board.body_motion, Some(BodyMotion::Stand));
            assert_eq!(board.kick_target, None);
        }
    }

    #[test]
    fn throw_in_taker_approaches_before_waiting_for_receiver() {
        let tree = sub_state_subtree();
        let ball = point![-1.5, 4.5];
        let mut board = restart(SubState::ThrowIn, ball);
        board.field_dimensions.length = 14.0;
        board.field_dimensions.width = 9.0;
        board.parameters.walking.path_planning.robot_radius = 0.4;
        board.parameters.walking.speed.kicking = 1.8;
        board.parameters.walking.speed.velocity_fade_distance = 3.0;
        board.field_dimensions.center_circle_diameter = 3.0;
        board.parameters.kicking.strategy.passing.minimum_distance = 4.0;
        board.parameters.kicking.approach_ball_standoff = 0.3;
        board.world_state.player_states.two = Some(TimeWrapper {
            time: board.world_state.now,
            inner: PlayerState {
                pose: point![-3.0, -0.5].into(),
                ..crate::test_support::player_state()
            },
        });
        board.world_state.robot.ground_to_field =
            Some(Pose2::new(ball - linear_algebra::vector![2.0, 1.0], 0.0).as_transform());
        assert!(restart_pass_receiver(&board).is_some());

        let (_, trace) = tree.tick_with_trace(&mut board);
        assert!(succeeded(&trace, "execute_ball_action"));
        assert!(!succeeded(&trace, "wait_for_restart_receiver"));
        assert!(matches!(board.body_motion, Some(BodyMotion::Walk { .. })));
        assert!(board.restart_pass_wait_started.is_none());

        board.world_state.robot.ground_to_field =
            Some(Pose2::new(ball - linear_algebra::vector![0.6, 0.0], 0.0).as_transform());
        let (_, trace) = tree.tick_with_trace(&mut board);
        assert!(succeeded(&trace, "wait_for_restart_receiver"));
        assert_eq!(board.body_motion, Some(BodyMotion::Stand));
        assert_eq!(board.restart_pass_wait_started, Some(board.world_state.now));
        board.last_motion_type = Some(MotionType::Stand);
        board.last_motion_switch_time = board.world_state.now;

        let since = board.world_state.now;
        for elapsed in [Duration::from_secs(5), Duration::from_secs(6)] {
            board.world_state.now = since + elapsed;
            board.world_state.player_states.two.as_mut().unwrap().time = board.world_state.now;
            let (_, trace) = tree.tick_with_trace(&mut board);
            assert!(!succeeded(&trace, "wait_for_restart_receiver"));
            assert_eq!(board.ball_action, Some(BallAction::Dribble));
            assert!(matches!(board.body_motion, Some(BodyMotion::Kick { .. })));
            assert_eq!(board.restart_pass_wait_started, Some(since));
        }

        board
            .world_state
            .filtered_game_controller_state
            .as_mut()
            .unwrap()
            .sub_state = None;
        board.update_restart_pass();
        assert!(board.restart_pass_wait_started.is_none());
        board
            .world_state
            .filtered_game_controller_state
            .as_mut()
            .unwrap()
            .sub_state = Some(SubState::ThrowIn);
        tree.tick_with_trace(&mut board);
        assert_eq!(board.body_motion, Some(BodyMotion::Stand));
        assert_eq!(board.restart_pass_wait_started, Some(board.world_state.now));
    }

    #[test]
    fn natural_corner_and_blocked_throw_in_stage_one_receiver_before_dribbling() {
        let tree = crate::tree::create_tree();
        for (state, ball, blocked) in [
            (SubState::CornerKick, point![7.0, 4.5], false),
            (SubState::CornerKick, point![7.0, -4.5], false),
            (SubState::CornerKick, point![7.0, 4.5], true),
            (SubState::ThrowIn, point![-1.5, 4.5], false),
            (SubState::ThrowIn, point![-1.5, 4.5], true),
        ] {
            let mut taker = restart(state, ball);
            taker.parameters.kicking.strategy.passing.minimum_distance = 4.0;
            taker.field_dimensions.length = 14.0;
            taker.field_dimensions.width = 9.0;
            taker.field_dimensions.ball_radius = 0.105;
            taker.parameters.walking.path_planning.robot_radius = 0.4;
            taker.parameters.walking.speed.kicking = 1.8;
            taker.parameters.walking.speed.velocity_fade_distance = 3.0;
            taker
                .parameters
                .walking
                .walk_and_stand
                .target_reached_thresholds = nalgebra::vector![0.05, 0.1];
            taker.field_dimensions.center_circle_diameter = 3.0;
            taker.player_number = PlayerNumber::Three;
            let taker_position = ball + linear_algebra::vector![0.22, 0.3];
            let toward_ball = ball - taker_position;
            let taker_pose = Pose2::new(taker_position, toward_ball.y().atan2(toward_ball.x()));
            taker.world_state.robot.ground_to_field = Some(taker_pose.as_transform());
            let receiver_position = point![-3.0, -0.5];
            taker.world_state.player_states.two = Some(TimeWrapper {
                time: taker.world_state.now,
                inner: PlayerState {
                    pose: Pose2::new(receiver_position, 0.0),
                    ..crate::test_support::player_state()
                },
            });
            let obstacle = (ball + receiver_position.coords()) / 2.0;
            if blocked {
                taker.world_state.obstacles.push(Obstacle::robot(
                    taker_pose.as_transform().inverse() * obstacle,
                    0.3,
                    0.3,
                ));
            }
            tree.tick_with_trace(&mut taker);
            assert_eq!(
                taker.body_motion,
                Some(BodyMotion::Stand),
                "{state:?}, blocked {blocked}"
            );
            assert!(taker.restart_pass_wait_started.is_some());
            let (number, target) = restart_pass_receiver(&taker).unwrap();
            assert_eq!(number, PlayerNumber::Two);
            assert!(target.x().abs() + 0.4 < 7.0);
            assert!(target.y().abs() + 0.4 < 4.5);
            let passing = &taker.parameters.kicking.strategy.passing;
            assert!(
                (target - ball).norm() - passing.receiver_standoff
                    >= passing.minimum_distance + 0.5,
                "{state:?} receiver must leave enough ball travel after its standoff"
            );
            if !blocked {
                let too_close = ball
                    + (target - ball).normalize()
                        * (passing.minimum_distance + passing.receiver_standoff + 0.1);
                assert!(!restart_pass_is_open(&taker, too_close));
            }
            let mut closed_lane = taker.clone();
            closed_lane.world_state.obstacles.push(Obstacle::robot(
                taker_pose.as_transform().inverse() * (ball + (target - ball) * 0.5),
                0.4,
                0.4,
            ));
            assert!(!restart_pass_is_open(&closed_lane, target));
            if state == SubState::CornerKick {
                let mut shallow = taker.clone();
                shallow.parameters.kicking.strategy.passing.minimum_distance = 0.0;
                let position = point![ball.x() - 1.5, ball.y().signum() * 3.0];
                shallow
                    .world_state
                    .player_states
                    .two
                    .as_mut()
                    .unwrap()
                    .inner
                    .pose = Pose2::new(position, 0.0);
                assert!(
                    !pass_openings(
                        ball,
                        position,
                        taker_pose.as_transform(),
                        &shallow.world_state.obstacles,
                        &shallow.field_dimensions,
                        &shallow.parameters.kicking.strategy.passing,
                        shallow.parameters.kicking.strategy.obstacle_clearance
                    )
                    .is_empty()
                );
                tree.tick_with_trace(&mut shallow);
                assert!(!matches!(
                    shallow.ball_action,
                    Some(BallAction::Pass { .. })
                ));
                assert_eq!(shallow.body_motion, Some(BodyMotion::Stand));
            } else {
                assert!(
                    (target.x() - ball.x()).abs()
                        >= 1.5 * taker.field_dimensions.center_circle_diameter / 2.0
                );
            }

            let mut receiver = taker.clone();
            receiver.player_number = PlayerNumber::Two;
            receiver.world_state.robot.ground_to_field =
                Some(Pose2::new(receiver_position, 0.0).as_transform());
            receiver.world_state.player_states.three = Some(TimeWrapper {
                time: receiver.world_state.now,
                inner: PlayerState {
                    pose: taker_pose,
                    ..crate::test_support::player_state()
                },
            });
            let transform = receiver.world_state.robot.ground_to_field.unwrap();
            receiver.world_state.obstacles = vec![Obstacle::robot(
                transform.inverse() * taker_position,
                0.4,
                0.4,
            )];
            if blocked {
                receiver.world_state.obstacles.push(Obstacle::robot(
                    transform.inverse() * obstacle,
                    0.3,
                    0.3,
                ));
            }
            tree.tick_with_trace(&mut receiver);
            assert!(
                matches!(receiver.body_motion, Some(BodyMotion::Walk { .. })),
                "{state:?}, blocked {blocked}"
            );
            let held_pose =
                Pose2::new(target, (ball.y() - target.y()).atan2(ball.x() - target.x()));
            receiver.world_state.robot.ground_to_field = Some(held_pose.as_transform());
            receiver.world_state.obstacles = vec![Obstacle::robot(
                held_pose.as_transform().inverse() * taker_position,
                0.4,
                0.4,
            )];
            let (_, trace) = tree.tick_with_trace(&mut receiver);
            if state == SubState::CornerKick {
                assert!(succeeded(&trace, "prepare_for_restart_pass"));
                assert_eq!(receiver.body_motion, Some(BodyMotion::Stand));
                assert!(target.x() <= ball.x() - 3.0);
            } else {
                assert!(succeeded(&trace, "walk_to_voronoi_position"));
            }

            let mut expired = taker.clone();
            expired.world_state.now = expired.world_state.now + Duration::from_secs(5);
            expired.world_state.player_states.two.as_mut().unwrap().time = expired.world_state.now;
            tree.tick_with_trace(&mut expired);
            assert_eq!(expired.ball_action, Some(BallAction::Dribble));
            expired
                .world_state
                .filtered_game_controller_state
                .as_mut()
                .unwrap()
                .sub_state = None;
            expired.update_restart_pass();
            assert!(expired.restart_pass_wait_started.is_none());

            taker
                .world_state
                .player_states
                .two
                .as_mut()
                .unwrap()
                .inner
                .pose = held_pose;
            tree.tick_with_trace(&mut taker);
            assert_eq!(
                taker.ball_action,
                Some(BallAction::Pass {
                    receiver: PlayerNumber::Two
                })
            );
            assert_eq!(taker.body_motion, Some(BodyMotion::Stand));
            let Some(OutgoingMessage::Hsl(hsl_network_messages::HulkMessage::State(message))) =
                taker.try_sending_state_message()
            else {
                panic!("staged receiver must receive the pass announcement");
            };
            receiver
                .world_state
                .player_states
                .three
                .as_mut()
                .unwrap()
                .inner
                .pass = message.pass;
            let (_, trace) = tree.tick_with_trace(&mut receiver);
            assert!(succeeded(&trace, "prepare_to_receive_pass"));
            assert!(matches!(receiver.body_motion,
                Some(BodyMotion::WalkWithVelocity { velocity, angular_velocity })
                if velocity == linear_algebra::Vector2::zeros() && angular_velocity == 0.0));
        }
    }

    #[test]
    fn unannounced_noncorner_support_preserves_voronoi_routes() {
        let tree = crate::tree::create_tree();
        for (state, ball, position, taker) in [
            (
                SubState::ThrowIn,
                point![-1.5, 4.5],
                point![-3.5, -0.5],
                point![-1.2, 4.2],
            ),
            (
                SubState::GoalKick,
                point![-5.5, 0.0],
                point![-1.5, -2.0],
                point![-5.2, 0.2],
            ),
            (
                SubState::IndirectFreeKick,
                point![0.0, 0.0],
                point![-3.0, -1.0],
                point![-0.3, 0.2],
            ),
        ] {
            let mut board = restart(state, ball);
            board.field_dimensions.length = 14.0;
            board.field_dimensions.width = 9.0;
            board.player_number = PlayerNumber::Two;
            let transform = Pose2::new(position, 0.0).as_transform();
            board.world_state.robot.ground_to_field = Some(transform);
            for (number, position) in [
                (PlayerNumber::One, point![-6.6, 0.0]),
                (PlayerNumber::Three, taker),
            ] {
                board.world_state.player_states[number] = Some(TimeWrapper {
                    time: board.world_state.now,
                    inner: PlayerState {
                        pose: Pose2::new(position, 0.0),
                        ..crate::test_support::player_state()
                    },
                });
            }
            board.parameters.walking.path_planning.robot_radius = 0.4;
            board.parameters.walking.speed.kicking = 1.8;
            board.parameters.walking.speed.velocity_fade_distance = 3.0;
            board.field_dimensions.center_circle_diameter = 3.0;
            board.parameters.voronoi.forward_weight = 0.2;
            board.parameters.voronoi.relative_ball_support_sigma =
                0.5 / board.field_dimensions.width;
            board.parameters.voronoi.centroid_anchor_weight = 0.6;
            board.parameters.voronoi.relative_centroid_anchor_sigma =
                1.0 / board.field_dimensions.width;
            assert!(restart_pass_is_open(&board, position), "{state:?}");
            for ball_weight in [0.0, 2.5] {
                let mut supporter = board.clone();
                supporter.parameters.voronoi.ball_weight = ball_weight;
                let (_, trace) = tree.tick_with_trace(&mut supporter);
                assert!(succeeded(&trace, "walk_to_voronoi_position"), "{state:?}");
                assert!(!succeeded(&trace, "prepare_for_restart_pass"), "{state:?}");
                let Some(BodyMotion::Walk { path, .. }) = supporter.body_motion else {
                    panic!("{state:?} support must move before any pass announcement");
                };
                let target = transform * path.end_point();
                assert!(target.inner.iter().all(|coordinate| coordinate.is_finite()));
                assert!(target.x().abs() <= 7.0 && target.y().abs() <= 4.5);
                assert_eq!(
                    supporter.voronoi_map.as_ref().unwrap().ownership_at(target),
                    Some(voronoi::Ownership::Robot(PlayerNumber::Two))
                );
            }
        }
    }

    #[test]
    fn five_player_restart_stages_only_one_blocked_receiver() {
        let tree = crate::tree::create_tree();
        let ball = point![7.0, 4.5];
        let mut board = restart(SubState::CornerKick, ball);
        board.field_dimensions.length = 14.0;
        board.field_dimensions.width = 9.0;
        board.parameters.walking.path_planning.robot_radius = 0.4;
        board.parameters.walking.speed.kicking = 1.8;
        board.parameters.walking.speed.velocity_fade_distance = 3.0;
        board.parameters.voronoi.ball_weight = 3.0;
        board.field_dimensions.center_circle_diameter = 3.0;
        board.parameters.voronoi.relative_ball_support_sigma = 1.0 / board.field_dimensions.width;
        board.parameters.voronoi.centroid_anchor_weight = 0.6;
        board.parameters.voronoi.relative_centroid_anchor_sigma =
            1.0 / board.field_dimensions.width;
        board.parameters.kicking.strategy.passing.minimum_distance = 4.0;
        for (number, position) in [
            (PlayerNumber::One, point![-6.6, 0.0]),
            (PlayerNumber::Two, point![-3.0, -0.5]),
            (PlayerNumber::Three, point![6.8, 4.2]),
            (PlayerNumber::Four, point![-5.0, -3.0]),
            (PlayerNumber::Five, point![-4.0, -2.0]),
        ] {
            board.world_state.player_states[number] = Some(TimeWrapper {
                time: board.world_state.now,
                inner: PlayerState {
                    pose: Pose2::new(position, 0.0),
                    ..crate::test_support::player_state()
                },
            });
        }
        let mut formation_targets = Vec::new();
        for number in [PlayerNumber::Two, PlayerNumber::Four, PlayerNumber::Five] {
            let mut supporter = board.clone();
            let transform = supporter.world_state.player_states[number]
                .as_ref()
                .unwrap()
                .inner
                .pose
                .as_transform();
            supporter.player_number = number;
            supporter.world_state.robot.ground_to_field = Some(transform);
            supporter.world_state.obstacles = vec![Obstacle::robot(
                transform.inverse() * point![2.0, 2.0],
                0.3,
                0.3,
            )];
            let (receiver, target) = restart_pass_receiver(&supporter).unwrap();
            assert_eq!(receiver, PlayerNumber::Two);
            let passing = &supporter.parameters.kicking.strategy.passing;
            assert!((target - ball).norm() - passing.receiver_standoff >= passing.minimum_distance);
            let (_, trace) = tree.tick_with_trace(&mut supporter);
            assert!(matches!(
                supporter.body_motion,
                Some(BodyMotion::Walk { .. })
            ));
            assert_eq!(
                succeeded(&trace, "prepare_for_restart_pass"),
                number == PlayerNumber::Two,
                "{number:?}"
            );
            assert_eq!(
                succeeded(&trace, "walk_to_voronoi_position"),
                number != PlayerNumber::Two,
                "{number:?}"
            );
            if number != PlayerNumber::Two {
                let Some(BodyMotion::Walk { path, .. }) = supporter.body_motion else {
                    unreachable!();
                };
                let target = transform * path.end_point();
                assert!(
                    (target - ball).norm() > supporter.field_dimensions.center_circle_diameter,
                    "{number:?} must retain its formation region: {target:?}"
                );
                formation_targets.push(target);
            }
        }
        assert!(
            (formation_targets[0] - formation_targets[1]).norm()
                > 2.0 * board.parameters.walking.path_planning.robot_radius
        );
    }
}
