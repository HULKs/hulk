use coordinate_systems::{Field, Ground};
use filtering::hysteresis::less_than_with_relative_hysteresis;
use geometry::{Distance, line_segment::LineSegment};
use hsl_network_messages::{PlayerNumber, SubState, Team};
use linear_algebra::{Isometry2, Point2, point};
use ros_z::Message;
use serde::{Deserialize, Serialize};
use types::{
    behavior_tree::Status,
    obstacles::{Obstacle, ObstacleKind},
};

use crate::{conditions::opponent_near_ball, node::Blackboard};

use super::{
    execution::goalward_target,
    geometry::{BALL_FIELD_BOUNDARY_TOLERANCE, goal_openings, pass_openings, travel_openings},
    kick_ball_position_in_ground, pass_receiver_position,
    passing::is_executing_announced_pass,
};

/// Tactical intent only. Execution chooses the target, approach, and physical motion.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Message)]
pub enum BallAction {
    Shoot {
        opening: LineSegment<Field>,
    },
    Pass {
        receiver: PlayerNumber,
    },
    Dribble,
    Clear,
    /// Explicit center shot, requiring an unobstructed corridor within shooting range.
    ShootAtGoalCenter,
}

/// Select ordinary striker intent. Clear `ball_action` when leaving that context.
/// Execution still needs to validate clear/dribble corridors and kick availability.
pub fn select_ball_action(blackboard: &mut Blackboard) -> Status {
    blackboard.ball_action = choose_ball_action(blackboard);
    if blackboard.ball_action.is_some() {
        Status::Success
    } else {
        Status::Failure
    }
}

fn choose_ball_action(blackboard: &Blackboard) -> Option<BallAction> {
    let world = &blackboard.world_state;
    let field = &blackboard.field_dimensions;
    let parameters = &blackboard.parameters.kicking.strategy;
    parameters.validate().ok()?;
    let previous = blackboard.ball_action;
    let ground_to_field = world.robot.ground_to_field?;
    let ball_in_ground = kick_ball_position_in_ground(blackboard, &ground_to_field)?;
    let ball = ground_to_field * ball_in_ground;
    let goal_x = field.length / 2.0;
    if !ball.x().is_finite()
        || !ball.y().is_finite()
        || !field.length.is_finite()
        || !field.width.is_finite()
        || field.length <= 0.0
        || field.width <= 0.0
        || ball.x().abs() > goal_x + BALL_FIELD_BOUNDARY_TOLERANCE
        || ball.y().abs() > field.width / 2.0 + BALL_FIELD_BOUNDARY_TOLERANCE
    {
        return None;
    }

    let restart = world
        .filtered_game_controller_state
        .as_ref()
        .filter(|game| game.kicking_team == Some(Team::Hulks))
        .and_then(|game| game.sub_state);
    let was_clearing = matches!(previous, Some(BallAction::Clear));
    let own_goal = LineSegment(
        point![-goal_x, -field.goal_inner_width / 2.0],
        point![-goal_x, field.goal_inner_width / 2.0],
    );
    let under_pressure = opponent_near_ball(
        blackboard,
        parameters.pressure_distance
            + if was_clearing {
                parameters.clear_distance_hysteresis
            } else {
                0.0
            },
    );
    // Our goal kick is a deliberate restart, including when retaining an old Clear intent.
    let can_clear = restart != Some(SubState::GoalKick)
        && under_pressure
        && travel_openings(
            ball,
            ground_to_field,
            &world.obstacles,
            field,
            parameters.maximum_shot_distance.min(goal_x),
            parameters.obstacle_clearance,
        )
        .iter()
        .any(|opening| opening.0.x().max(opening.1.x()) > ball.x());
    if can_clear
        && less_than_with_relative_hysteresis(
            was_clearing,
            own_goal.distance_to(ball),
            parameters.clear_goal_distance,
            0.0..=parameters.clear_distance_hysteresis,
        )
    {
        return Some(BallAction::Clear);
    }

    let pass = choose_pass(blackboard, ball, ground_to_field);
    let keeping_pass = matches!(pass, Some(BallAction::Pass { receiver })
        if is_executing_announced_pass(blackboard, receiver));
    if pass.is_some()
        && (restart.is_some_and(|state| state != SubState::PenaltyKick) || keeping_pass)
    {
        return pass;
    }
    let can_shoot = !matches!(
        restart,
        Some(SubState::ThrowIn | SubState::IndirectFreeKick)
    );
    let range = parameters.maximum_shot_distance;
    let shot = if can_shoot && range.is_finite() && range > goal_x - ball.x() {
        // Clip regions to reachable goal points before rating their angular width.
        let reachable_y = (range.powi(2) - (goal_x - ball.x()).powi(2)).sqrt();
        let mut best = None;
        let mut retained = None;
        for opening in goal_openings(
            ball,
            ground_to_field,
            &world.obstacles,
            field,
            parameters.obstacle_clearance,
        ) {
            let lower = opening.0.y().max(ball.y() - reachable_y);
            let upper = opening.1.y().min(ball.y() + reachable_y);
            if lower >= upper {
                continue;
            }
            let opening = LineSegment(point![goal_x, lower], point![goal_x, upper]);
            let angle = LineSegment(ball, opening.0).angle(LineSegment(ball, opening.1));
            if angle < parameters.minimum_shot_angle {
                continue;
            }
            if best.is_none_or(|(_, best_angle)| angle > best_angle) {
                best = Some((opening, angle));
            }
            if let Some(BallAction::Shoot { opening: previous }) = previous
                && previous.0.x() == goal_x
                && (lower..upper).contains(&previous.center().y())
            {
                retained = Some((opening, angle));
            }
        }
        best.map(|(opening, angle)| {
            retained
                .filter(|(_, old_angle)| angle <= old_angle + parameters.shot_angle_hysteresis)
                .unwrap_or((opening, angle))
        })
    } else {
        None
    };

    if let Some((opening, angle)) = shot {
        if angle >= parameters.preferred_shot_angle {
            return Some(BallAction::Shoot { opening });
        }
        if let Some(BallAction::Pass { receiver }) = pass
            && let Some(position) = pass_receiver_position(blackboard, ball, receiver)
            && receiver_has_space(blackboard, position, ground_to_field)
            && receiver_goal_angle(blackboard, position, ground_to_field)
                > angle + parameters.shot_angle_hysteresis
        {
            return pass;
        }
        let goal = point![goal_x, 0.0];
        if !under_pressure
            && travel_openings(
                ball,
                ground_to_field,
                &world.obstacles,
                field,
                parameters.maximum_dribble_distance,
                parameters.obstacle_clearance,
            )
            .into_iter()
            .map(|opening| goalward_target(ball, goal, opening))
            .min_by(|a, b| {
                (a - ball)
                    .angle(&(goal - ball))
                    .total_cmp(&(b - ball).angle(&(goal - ball)))
            })
            .is_some_and(|target| {
                target.x() > ball.x()
                    && (goal - target).norm() < (goal - ball).norm()
                    && visible_goal_angle(blackboard, target, ground_to_field, &world.obstacles)
                        > angle
            })
        {
            return Some(BallAction::Dribble);
        }
        // A narrow, clear shot still beats waiting when neither pass nor dribble improves it.
        return Some(BallAction::Shoot { opening });
    }

    if let Some(pass) = pass {
        return Some(pass);
    }

    if can_shoot && can_clear {
        return Some(BallAction::Clear);
    }

    (!travel_openings(
        ball,
        ground_to_field,
        &world.obstacles,
        field,
        parameters.maximum_dribble_distance,
        parameters.obstacle_clearance,
    )
    .is_empty())
    .then_some(BallAction::Dribble)
}

fn choose_pass(
    blackboard: &Blackboard,
    ball: Point2<Field>,
    ground_to_field: Isometry2<Ground, Field>,
) -> Option<BallAction> {
    if !blackboard.has_team_message_budget() {
        return None;
    }
    let world = &blackboard.world_state;
    let field = &blackboard.field_dimensions;
    let parameters = &blackboard.parameters.kicking.strategy;
    let previous = blackboard.ball_action;
    let mut best: Option<(PlayerNumber, f32, f32, bool)> = None;
    let mut retained = None;
    for (receiver, _) in world.player_states.iter() {
        let Some(position) = pass_receiver_position(blackboard, ball, receiver) else {
            continue;
        };
        let executing_pass = is_executing_announced_pass(blackboard, receiver);
        if world
            .filtered_game_controller_state
            .as_ref()
            .is_some_and(|game| {
                game.kicking_team == Some(Team::Hulks)
                    && matches!(
                        game.sub_state,
                        Some(SubState::CornerKick | SubState::ThrowIn)
                    )
            })
            && !executing_pass
            && !crate::substates::restart_pass_is_open(blackboard, position)
        {
            continue;
        }
        let mut passing = parameters.passing.clone();
        if executing_pass {
            passing.minimum_distance = 0.0;
        }
        if pass_openings(
            ball,
            position,
            ground_to_field,
            &world.obstacles,
            field,
            &passing,
            parameters.obstacle_clearance,
        )
        .is_empty()
        {
            continue;
        }
        if executing_pass {
            return Some(BallAction::Pass { receiver });
        }
        let progress = position.x() - ball.x();
        let angle = receiver_goal_angle(blackboard, position, ground_to_field);
        let space = receiver_has_space(blackboard, position, ground_to_field);
        if best.is_none_or(|(_, best_angle, best_progress, best_space)| {
            (space && !best_space)
                || (space == best_space
                    && (angle > best_angle + parameters.shot_angle_hysteresis
                        || (angle + parameters.shot_angle_hysteresis >= best_angle
                            && progress > best_progress)))
        }) {
            best = Some((receiver, angle, progress, space));
        }
        if previous == Some(BallAction::Pass { receiver }) {
            retained = Some((receiver, angle, progress, space));
        }
    }
    best.map(|(receiver, angle, progress, space)| {
        let receiver = retained
            .filter(|(_, old_angle, old_progress, old_space)| {
                (!space || *old_space)
                    && angle <= old_angle + parameters.shot_angle_hysteresis
                    && progress <= old_progress + parameters.passing.selection_hysteresis
            })
            .map_or(receiver, |(receiver, _, _, _)| receiver);
        BallAction::Pass { receiver }
    })
}

fn visible_goal_angle(
    blackboard: &Blackboard,
    position: Point2<Field>,
    ground_to_field: Isometry2<Ground, Field>,
    obstacles: &[Obstacle],
) -> f32 {
    goal_openings(
        position,
        ground_to_field,
        obstacles,
        &blackboard.field_dimensions,
        blackboard.parameters.kicking.strategy.obstacle_clearance,
    )
    .into_iter()
    .map(|opening| LineSegment(position, opening.0).angle(LineSegment(position, opening.1)))
    .fold(0.0, f32::max)
}

fn is_receiver_obstacle(
    obstacle: &Obstacle,
    receiver: Point2<Field>,
    ground_to_field: Isometry2<Ground, Field>,
) -> bool {
    matches!(
        obstacle.kind,
        ObstacleKind::Robot | ObstacleKind::Person | ObstacleKind::Unknown
    ) && (ground_to_field * obstacle.position - receiver).norm() <= obstacle.radius_at_foot_height
}

fn receiver_goal_angle(
    blackboard: &Blackboard,
    receiver: Point2<Field>,
    ground_to_field: Isometry2<Ground, Field>,
) -> f32 {
    let obstacles: Vec<_> = blackboard
        .world_state
        .obstacles
        .iter()
        .filter(|obstacle| !is_receiver_obstacle(obstacle, receiver, ground_to_field))
        .copied()
        .collect();
    visible_goal_angle(blackboard, receiver, ground_to_field, &obstacles)
}

fn receiver_has_space(
    blackboard: &Blackboard,
    receiver: Point2<Field>,
    ground_to_field: Isometry2<Ground, Field>,
) -> bool {
    blackboard.world_state.obstacles.iter().all(|obstacle| {
        !matches!(
            obstacle.kind,
            ObstacleKind::Robot | ObstacleKind::Person | ObstacleKind::Unknown
        ) || is_receiver_obstacle(obstacle, receiver, ground_to_field)
            || (ground_to_field * obstacle.position - receiver).norm()
                - obstacle.radius_at_foot_height
                > blackboard.parameters.kicking.strategy.pressure_distance
    })
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use hsl_network_messages::Penalty;
    use linear_algebra::{Pose2, vector};
    use types::{
        ball_position::BallPosition,
        fall_detection::{FallDetection, Posture},
        field_dimensions::{FieldDimensions, Half, Side},
        motion_command::BodyMotion,
        obstacles::Obstacle,
        parameters::{KickStrategyParameters, PassingParameters},
        time_wrapper::TimeWrapper,
        world_state::{BallState, PlayerState},
    };

    use crate::{
        kick::{
            execution::execute_ball_action,
            passing::{is_pass_ball_arriving, is_pass_receiver},
        },
        node::LastBall,
        tree::{create_tree, striker_subtree},
    };

    use super::*;

    fn blackboard() -> Blackboard {
        let mut blackboard = Blackboard {
            field_dimensions: FieldDimensions::SPL_2025,
            ball: Some(LastBall {
                position: point![2.0, 0.0],
                velocity: Default::default(),
                age: Default::default(),
                field_side: Side::Left,
                source: types::world_state::BallSource::Own,
            }),
            ..Default::default()
        };
        blackboard.world_state.robot.ground_to_field = Some(Default::default());
        blackboard.parameters.kicking.strategy = KickStrategyParameters {
            maximum_shot_distance: 3.0,
            maximum_dribble_distance: 3.0,
            obstacle_clearance: 0.05,
            minimum_shot_angle: 0.02,
            preferred_shot_angle: 0.0,
            shot_angle_hysteresis: 0.05,
            pressure_distance: 0.8,
            clear_goal_distance: 1.5,
            clear_distance_hysteresis: 0.2,
            passing: Default::default(),
        };
        blackboard
    }

    fn passing_blackboard() -> Blackboard {
        let mut blackboard = blackboard();
        blackboard.world_state.robot.primary_state = types::primary_state::PrimaryState::Playing;
        blackboard.world_state.filtered_game_controller_state = Some(
            types::filtered_game_controller_state::FilteredGameControllerState {
                remaining_number_of_messages: 100,
                remaining_time_in_half: Duration::from_millis(4800),
                ..crate::test_support::filtered_game_controller_state()
            },
        );
        blackboard
            .parameters
            .network
            .remaining_amount_of_messages_to_stop_sending = 20;
        blackboard.ball.as_mut().unwrap().position = point![0.0, 0.0];
        blackboard.parameters.kicking.strategy.passing = PassingParameters {
            maximum_distance: 3.0,
            minimum_distance: 0.0,
            minimum_progress: 0.5,
            receiver_standoff: 0.5,
            target_half_width: 0.2,
            selection_hysteresis: 0.25,
            maximum_receiver_age: Duration::from_millis(500),
        };
        blackboard.parameters.kicking.kick_activation_distance = 3.0;
        blackboard.parameters.kicking.target_speed = 3.4;
        blackboard.world_state.player_states.two = Some(TimeWrapper {
            time: blackboard.world_state.now,
            inner: PlayerState {
                pose: Pose2::new(point![2.0, 0.0], 0.0),
                ..crate::test_support::player_state()
            },
        });
        blackboard.world_state.obstacles = vec![Obstacle::robot(point![2.0, 0.0], 0.2, 0.25)];
        blackboard
    }

    #[test]
    fn narrow_goal_openings_prefer_useful_passes_or_short_forward_dribbles() {
        let mut board = passing_blackboard();
        board.field_dimensions.length = 14.0;
        board.field_dimensions.width = 9.0;
        board.field_dimensions.goal_inner_width = 2.4;
        board.field_dimensions.ball_radius = 0.105;
        board.parameters.kicking.strategy.maximum_shot_distance = 12.0;
        board.parameters.kicking.strategy.maximum_dribble_distance = 1.0;
        board.parameters.kicking.strategy.preferred_shot_angle = 0.12;
        board.parameters.kicking.strategy.passing.maximum_distance = 8.0;
        board.parameters.kicking.strategy.passing.minimum_progress = 0.0;
        let keeper = Obstacle::robot(point![6.6, 0.0], 0.2, 0.2);
        for (receiver, expected) in [
            (
                point![2.5, 2.0],
                BallAction::Pass {
                    receiver: PlayerNumber::Two,
                },
            ),
            (point![1.0, 0.5], BallAction::Dribble),
            (point![0.0, 2.5], BallAction::Dribble),
        ] {
            board
                .world_state
                .player_states
                .two
                .as_mut()
                .unwrap()
                .inner
                .pose = Pose2::new(receiver, 0.0);
            board.world_state.obstacles = vec![
                keeper,
                Obstacle::robot(
                    board.world_state.robot.ground_to_field.unwrap().inverse() * receiver,
                    0.2,
                    0.2,
                ),
            ];
            board.ball_action = None;
            select_ball_action(&mut board);
            assert_eq!(
                board.ball_action,
                Some(expected),
                "receiver at {receiver:?}"
            );
        }

        // A wider pass still needs a clear corridor and room to control the reception.
        for opponent in [point![0.0, 1.2], point![0.0, 3.2]] {
            board.world_state.obstacles = vec![
                keeper,
                Obstacle::robot(point![0.0, 2.5], 0.2, 0.2),
                Obstacle::robot(opponent, 0.2, 0.2),
            ];
            board.ball_action = None;
            select_ball_action(&mut board);
            assert_eq!(board.ball_action, Some(BallAction::Dribble));
        }

        board.world_state.player_states.two = None;
        board.world_state.obstacles = vec![keeper];
        board.ball_action = None;
        select_ball_action(&mut board);
        assert_eq!(board.ball_action, Some(BallAction::Dribble));

        // Do not wait indefinitely when no improving dribble or pass is available.
        board.parameters.kicking.strategy.maximum_dribble_distance = 0.0;
        select_ball_action(&mut board);
        assert!(matches!(board.ball_action, Some(BallAction::Shoot { .. })));

        // Close openings and an unguarded goal remain immediate shots.
        board.parameters.kicking.strategy.maximum_dribble_distance = 1.0;
        board.ball.as_mut().unwrap().position = point![4.0, 0.0];
        select_ball_action(&mut board);
        assert!(matches!(board.ball_action, Some(BallAction::Shoot { .. })));
        board.ball.as_mut().unwrap().position = point![0.0, 0.0];
        board.world_state.obstacles.clear();
        select_ball_action(&mut board);
        assert!(matches!(board.ball_action, Some(BallAction::Shoot { .. })));

        // The production preference lets a clear midfield opening win over a useful pass.
        board.parameters.kicking.strategy.preferred_shot_angle = 0.06;
        let receiver = point![2.5, 2.0];
        board.world_state.player_states.two = Some(TimeWrapper {
            time: board.world_state.now,
            inner: PlayerState {
                pose: receiver.into(),
                ..crate::test_support::player_state()
            },
        });
        board.world_state.obstacles = vec![
            keeper,
            Obstacle::robot(
                board.world_state.robot.ground_to_field.unwrap().inverse() * receiver,
                0.2,
                0.2,
            ),
        ];
        select_ball_action(&mut board);
        assert!(matches!(board.ball_action, Some(BallAction::Shoot { .. })));
    }

    #[test]
    fn announced_receivers_block_then_shoot_around_a_visible_center_keeper() {
        for sub_state in [None, Some(SubState::CornerKick), Some(SubState::ThrowIn)] {
            let mut board = passing_blackboard();
            board.field_dimensions.length = 14.0;
            board.field_dimensions.width = 9.0;
            board.field_dimensions.goal_inner_width = 2.4;
            board.field_dimensions.ball_radius = 0.105;
            board.parameters.goalkeeper.player_number = PlayerNumber::One;
            board.parameters.kicking.strategy.maximum_shot_distance = 12.0;
            board.parameters.kicking.kick_activation_distance = 1.3;
            board.parameters.kicking.approach_ball_standoff = 0.3;
            board.parameters.ball.last_ball_timeout = Duration::from_secs(1);
            board.parameters.ball.pursuit_lookahead = Duration::from_millis(500);
            board.parameters.ball.interception.minimum_ball_velocity = 0.3;
            board.parameters.walking.path_planning.robot_radius = 0.4;
            board.last_motion_type = Some(types::motion_type::MotionType::Walk);
            board.player_number = PlayerNumber::Three;
            let transform = Pose2::new(point![-0.45, -2.35], 0.45_f32.atan2(0.05)).as_transform();
            board.world_state.robot.ground_to_field = Some(transform);
            board.world_state.fall_detection = Some(FallDetection {
                time: board.world_state.now,
                valid_until: board.world_state.now + Duration::from_secs(10),
                posture: Posture::Upright,
            });
            board.ball.as_mut().unwrap().position = point![-0.4, -1.9];
            board.ball.as_mut().unwrap().velocity = linear_algebra::vector![0.0, -1.0];
            board.world_state.player_states.two.as_mut().unwrap().inner = PlayerState {
                pose: Pose2::new(point![-0.2, 0.0], 0.0),
                pass: Some(hsl_network_messages::PassIntent {
                    receiver: PlayerNumber::Three,
                    target: point![-0.4, -1.85],
                }),
                ..crate::test_support::player_state()
            };
            let keeper = point![6.7, 0.0];
            board.world_state.obstacles =
                vec![Obstacle::robot(transform.inverse() * keeper, 0.2, 0.25)];
            let game = board
                .world_state
                .filtered_game_controller_state
                .as_mut()
                .unwrap();
            game.kicking_team = Some(Team::Hulks);
            game.sub_state = sub_state;
            // The receiver must block even while GameController still holds our restart.
            create_tree().tick_with_trace(&mut board);
            assert!(matches!(
                board.body_motion,
                Some(BodyMotion::Walk { .. } | BodyMotion::WalkWithVelocity { .. })
            ));
            board
                .world_state
                .filtered_game_controller_state
                .as_mut()
                .unwrap()
                .sub_state = None;
            board.ball.as_mut().unwrap().velocity = Default::default();
            board.world_state.now = board.world_state.now + Duration::from_millis(1);
            board.ball.as_mut().unwrap().age = board.world_state.now;
            board.kick_target = None;
            board.body_motion = None;
            create_tree().tick_with_trace(&mut board);
            assert!(matches!(board.ball_action, Some(BallAction::Shoot { .. })));
            let target = transform * board.kick_target.unwrap();
            assert!(target.y().abs() > 0.1);
            assert!(
                LineSegment(board.ball.as_ref().unwrap().position, target).distance_to(keeper)
                    > 0.2
                        + board.field_dimensions.ball_radius
                        + board.parameters.kicking.strategy.obstacle_clearance
            );
            assert!(matches!(board.body_motion, Some(BodyMotion::Kick { .. })));

            board.world_state.obstacles[0].radius_at_foot_height = 2.0;
            board.kick_target = None;
            board.body_motion = None;
            create_tree().tick_with_trace(&mut board);
            assert!(!matches!(
                board.ball_action,
                Some(BallAction::Shoot { .. } | BallAction::ShootAtGoalCenter)
            ));
        }
    }

    #[test]
    fn shoots_from_large_field_midfield_only_with_a_clear_goal_corridor() {
        let mut blackboard = blackboard();
        blackboard.field_dimensions.length = 22.0;
        blackboard.field_dimensions.width = 14.0;
        blackboard.field_dimensions.goal_inner_width = 2.6;
        blackboard.ball.as_mut().unwrap().position = point![0.0, 0.0];
        blackboard.parameters.kicking.strategy.maximum_shot_distance = 12.0;
        select_ball_action(&mut blackboard);
        assert!(matches!(
            blackboard.ball_action,
            Some(BallAction::Shoot { .. })
        ));

        blackboard.world_state.obstacles = vec![Obstacle::robot(point![5.5, 0.0], 1.5, 1.5)];
        select_ball_action(&mut blackboard);
        assert_eq!(blackboard.ball_action, Some(BallAction::Dribble));

        blackboard.world_state.obstacles.clear();
        blackboard.ball.as_mut().unwrap().position = point![-2.0, 0.0];
        select_ball_action(&mut blackboard);
        assert_eq!(blackboard.ball_action, Some(BallAction::Dribble));
    }

    #[test]
    fn starts_midfield_shots_around_a_center_keeper_with_filtered_posts() {
        for ball_x in [0.0, -1.0, -2.0, -3.0] {
            let mut board = blackboard();
            let field = &mut board.field_dimensions;
            field.length = 14.0;
            field.width = 9.0;
            field.goal_inner_width = 2.4;
            field.ball_radius = 0.105;
            board.parameters.kicking.strategy.maximum_shot_distance = 12.0;
            let ball = point![ball_x, 0.0];
            let keeper = point![6.5, 0.0];
            board.ball.as_mut().unwrap().position = ball;
            board.world_state.obstacles = vec![Obstacle::robot(
                board.world_state.robot.ground_to_field.unwrap().inverse() * keeper,
                0.2,
                0.2,
            )];
            // The filtering profile publishes larger post obstacles than the physical posts.
            for y in [-1.25, 1.25] {
                board
                    .world_state
                    .obstacles
                    .push(Obstacle::goal_post(point![7.025, y], 0.2));
            }

            select_ball_action(&mut board);
            let Some(BallAction::Shoot { opening }) = board.ball_action else {
                panic!("expected a fresh shot from x={ball_x}");
            };
            assert!(opening.0.y() * opening.1.y() > 0.0);
            let target = crate::kick::execution::resolve_target(&board).unwrap();
            assert!(LineSegment(ball, target).distance_to(keeper) > 0.355);

            board.world_state.obstacles[0].radius_at_foot_height = 2.0;
            board.ball_action = None;
            select_ball_action(&mut board);
            assert!(!matches!(board.ball_action, Some(BallAction::Shoot { .. })));
        }
    }

    #[test]
    fn distant_pass_reselects_stale_receiver_and_announces_new_target() {
        let mut board = passing_blackboard();
        board.field_dimensions.length = 22.0;
        board.field_dimensions.width = 14.0;
        board.field_dimensions.goal_inner_width = 2.6;
        board.parameters.kicking.strategy.maximum_shot_distance = 12.0;
        board.parameters.kicking.strategy.passing.maximum_distance = 8.0;
        board
            .parameters
            .kicking
            .strategy
            .passing
            .maximum_receiver_age = Duration::from_secs(3);
        board.parameters.kicking.kick_activation_distance = 1.3;
        board.ball.as_mut().unwrap().position = point![-4.0, 0.0];
        let transform = Pose2::new(point![-4.6, 0.0], 0.0).as_transform();
        board.world_state.robot.ground_to_field = Some(transform);
        board.world_state.now = board.world_state.now + Duration::from_secs(2);
        board
            .world_state
            .player_states
            .two
            .as_mut()
            .unwrap()
            .inner
            .pose = Pose2::new(point![2.0, 1.0], 0.0);
        board.world_state.obstacles.clear();
        select_ball_action(&mut board);
        assert_eq!(
            board.ball_action,
            Some(BallAction::Pass {
                receiver: PlayerNumber::Two
            })
        );
        execute_ball_action(&mut board);
        assert!((board.kick_target.unwrap() - point![0.6, 0.0]).norm() > 5.0);
        assert!(board.try_sending_state_message().is_some());

        board.world_state.now = board.world_state.now + Duration::from_secs(1);
        board.world_state.player_states.four = Some(TimeWrapper {
            time: board.world_state.now,
            inner: PlayerState {
                pose: Pose2::new(point![1.0, -1.0], 0.0),
                ..crate::test_support::player_state()
            },
        });
        execute_ball_action(&mut board);
        assert_eq!(
            board.ball_action,
            Some(BallAction::Pass {
                receiver: PlayerNumber::Four
            })
        );
        assert_eq!(board.body_motion, Some(BodyMotion::Stand));
        assert!(board.last_announced_pass.is_none());
        assert!(board.try_sending_state_message().is_some());
        board.world_state.now = board.world_state.now + Duration::from_millis(150);
        execute_ball_action(&mut board);
        assert!(matches!(
            board.body_motion,
            Some(BodyMotion::Kick {
                strong: false,
                soft: false,
                target_speed: 3.4,
                ..
            })
        ));
    }

    #[test]
    fn pass_refines_receiver_region_and_revalidates_before_kicking() {
        let mut blackboard = passing_blackboard();
        let transform =
            linear_algebra::Isometry2::from_parts(linear_algebra::vector![-0.3, -0.2], 0.6);
        blackboard.world_state.robot.ground_to_field = Some(transform);
        blackboard.world_state.obstacles = [
            Obstacle::robot(transform.inverse() * point![2.0, 0.0], 0.2, 0.25),
            Obstacle::robot(transform.inverse() * point![0.8, -0.04], 0.01, 0.01),
        ]
        .to_vec();
        blackboard.parameters.kicking.allow_strong_kicks = true;
        blackboard
            .parameters
            .kicking
            .strong_kick_min_target_distance = 0.1;
        blackboard.parameters.kicking.quick = true;
        select_ball_action(&mut blackboard);
        assert_eq!(
            blackboard.ball_action,
            Some(BallAction::Pass {
                receiver: PlayerNumber::Two
            })
        );
        execute_ball_action(&mut blackboard);
        let target = transform * blackboard.kick_target.unwrap();
        assert!((target.x() - 1.5).abs() < 1e-5);
        assert!(target.y() > 0.0 && target.y() < 0.2);
        assert_eq!(blackboard.body_motion, Some(BodyMotion::Stand));
        assert!(blackboard.try_sending_state_message().is_some());
        blackboard.world_state.now = blackboard.world_state.now + Duration::from_millis(150);
        execute_ball_action(&mut blackboard);
        let Some(BodyMotion::Kick {
            kick_direction,
            soft: false,
            strong: false,
            quick: false,
            target_speed: 3.4,
            ..
        }) = blackboard.body_motion
        else {
            panic!("expected controlled pass")
        };
        let direction = transform * kick_direction.as_unit_vector();
        assert!((direction - target.coords().normalize()).norm() < 1e-5);
        for obstacle in &blackboard.world_state.obstacles {
            assert!(
                LineSegment(point![0.0, 0.0], target).distance_to(transform * obstacle.position)
                    > obstacle.radius_at_foot_height + 0.1
            );
        }

        let receiver = point![2.0, 0.4];
        blackboard
            .world_state
            .player_states
            .two
            .as_mut()
            .unwrap()
            .inner
            .pose = Pose2::new(receiver, 0.0);
        blackboard.world_state.obstacles[0].position = transform.inverse() * receiver;
        execute_ball_action(&mut blackboard);
        let refined = transform * blackboard.kick_target.unwrap();
        assert!(refined.y() > target.y());
        assert!((refined - receiver).norm() <= 0.54);

        blackboard.world_state.obstacles.push(Obstacle::robot(
            transform.inverse() * point![0.8, 0.0],
            0.4,
            0.4,
        ));
        execute_ball_action(&mut blackboard);
        assert!(matches!(
            blackboard.body_motion,
            Some(BodyMotion::Kick {
                strong: true,
                quick: true,
                soft: false,
                ..
            })
        ));
        assert_eq!(blackboard.ball_action, Some(BallAction::Clear));
        let target = transform * blackboard.kick_target.unwrap();
        for obstacle in &blackboard.world_state.obstacles {
            assert!(
                LineSegment(point![0.0, 0.0], target).distance_to(transform * obstacle.position)
                    > obstacle.radius_at_foot_height + 0.1
            );
        }
        assert!(blackboard.last_announced_pass.is_none());
    }

    #[test]
    fn pass_rejects_unavailable_receivers_and_unsafe_corridors() {
        type InvalidateReceiver = fn(&mut Blackboard);
        let cases: [(&str, InvalidateReceiver); 13] = [
            ("goalkeeper", |b| {
                b.parameters.goalkeeper.player_number = PlayerNumber::Two
            }),
            ("message budget", |b| {
                b.world_state
                    .filtered_game_controller_state
                    .as_mut()
                    .unwrap()
                    .remaining_number_of_messages = 19;
            }),
            ("disabled", |b| {
                b.parameters.kicking.strategy.passing.maximum_distance = 0.0
            }),
            ("short", |b| {
                b.parameters.kicking.strategy.passing.minimum_distance = 1.6
            }),
            ("missing", |b| b.world_state.player_states.two = None),
            ("self", |b| b.player_number = PlayerNumber::Two),
            ("stale", |b| {
                b.world_state.now = b.world_state.now + Duration::from_millis(500)
            }),
            ("future", |b| {
                b.world_state.player_states.two.as_mut().unwrap().time =
                    b.world_state.now + Duration::from_nanos(1)
            }),
            ("penalized", |b| {
                let mut game = crate::test_support::filtered_game_controller_state();
                game.penalties.two = Some(Penalty::PickUp {
                    remaining: Duration::from_secs(1),
                });
                b.world_state.filtered_game_controller_state = Some(game);
            }),
            ("behind", |b| {
                b.world_state.player_states.two.as_mut().unwrap().inner.pose =
                    Pose2::new(point![-1.0, 0.0], 0.0)
            }),
            ("range", |b| {
                b.world_state.player_states.two.as_mut().unwrap().inner.pose =
                    Pose2::new(point![4.1, 0.0], 0.0)
            }),
            ("blocked", |b| {
                b.world_state
                    .obstacles
                    .push(Obstacle::robot(point![0.7, 0.0], 0.4, 0.4))
            }),
            ("opponent near receiver", |b| {
                b.world_state
                    .obstacles
                    .push(Obstacle::robot(point![1.65, 0.0], 0.2, 0.2))
            }),
        ];
        for (name, invalidate) in cases {
            let mut blackboard = passing_blackboard();
            invalidate(&mut blackboard);
            select_ball_action(&mut blackboard);
            assert_eq!(blackboard.ball_action, Some(BallAction::Dribble), "{name}");
            blackboard.ball_action = Some(BallAction::Pass {
                receiver: PlayerNumber::Two,
            });
            execute_ball_action(&mut blackboard);
            assert!(
                matches!(
                    blackboard.body_motion,
                    Some(BodyMotion::Kick {
                        strong: false,
                        soft: false,
                        ..
                    })
                ),
                "{name}"
            );
            assert_eq!(blackboard.ball_action, Some(BallAction::Dribble), "{name}");
        }
    }

    #[test]
    fn new_pass_announces_immediately_and_only_set_plays_wait_for_preparation() {
        let mut approaching = passing_blackboard();
        approaching.world_state.robot.ground_to_field =
            Some(Pose2::new(point![-2.0, 0.0], 0.0).as_transform());
        approaching.world_state.obstacles[0].position = point![4.0, 0.0];
        approaching.parameters.kicking.kick_activation_distance = 1.3;
        approaching.parameters.kicking.kick_activation_hysteresis = 0.3;
        approaching.parameters.walking.speed.kicking = 1.8;
        approaching.parameters.walking.speed.velocity_fade_distance = 3.0;
        select_ball_action(&mut approaching);
        execute_ball_action(&mut approaching);
        assert!(matches!(
            approaching.ball_action,
            Some(BallAction::Pass { .. })
        ));
        assert!(!approaching.last_close_enough_to_kick);
        let Some(types::messages::OutgoingMessage::Hsl(hsl_network_messages::HulkMessage::State(
            message,
        ))) = approaching.try_sending_state_message()
        else {
            panic!("expected ordinary state update while approaching");
        };
        assert!(message.pass.is_none());
        approaching.world_state.robot.ground_to_field =
            Some(Pose2::new(point![-1.5, 0.0], 0.0).as_transform());
        approaching.world_state.obstacles[0].position = point![3.5, 0.0];
        execute_ball_action(&mut approaching);
        assert!(matches!(
            approaching.body_motion,
            Some(BodyMotion::Walk { .. })
        ));
        assert!(approaching.try_sending_state_message().is_some());
        approaching.world_state.robot.ground_to_field =
            Some(Pose2::new(point![-0.6, 0.0], 0.0).as_transform());
        approaching.world_state.obstacles[0].position = point![2.6, 0.0];
        execute_ball_action(&mut approaching);
        assert!(matches!(
            approaching.body_motion,
            Some(BodyMotion::Kick { .. })
        ));

        for sub_state in [None, Some(SubState::ThrowIn)] {
            let mut board = passing_blackboard();
            board.parameters.kicking.strategy.passing.minimum_distance = 1.4;
            board
                .world_state
                .player_states
                .two
                .as_mut()
                .unwrap()
                .inner
                .pose = point![3.0, 0.0].into();
            board.world_state.obstacles[0].position = point![3.0, 0.0];
            board.last_sent_hsl_message_time = Some(board.world_state.now);
            let game = board
                .world_state
                .filtered_game_controller_state
                .as_mut()
                .unwrap();
            game.kicking_team = Some(Team::Hulks);
            game.sub_state = sub_state;
            select_ball_action(&mut board);
            execute_ball_action(&mut board);
            assert_eq!(board.body_motion, Some(BodyMotion::Stand));
            let Some(types::messages::OutgoingMessage::Hsl(
                hsl_network_messages::HulkMessage::State(message),
            )) = board.try_sending_state_message()
            else {
                panic!("new pass must announce without waiting for the normal cooldown");
            };
            let pass = message.pass.unwrap();
            assert_eq!(pass.receiver, PlayerNumber::Two);
            let transform = board.world_state.robot.ground_to_field.unwrap();
            assert_eq!(
                Some(pass.target),
                board.kick_target.map(|target| transform * target)
            );
            execute_ball_action(&mut board);
            if sub_state.is_some() {
                assert_eq!(board.body_motion, Some(BodyMotion::Stand));
                board.world_state.now = board.world_state.now + Duration::from_millis(149);
                execute_ball_action(&mut board);
                assert_eq!(board.body_motion, Some(BodyMotion::Stand));
                board.world_state.now = board.world_state.now + Duration::from_millis(1);
                execute_ball_action(&mut board);
            }
            assert!(matches!(board.body_motion, Some(BodyMotion::Kick { .. })));
            assert!(board.try_sending_state_message().is_none());

            // The remaining distance shrinks during an already announced kick.
            board.last_motion_type = Some(types::motion_type::MotionType::Kick);
            board.last_motion_command =
                Some(types::motion_command::MotionCommand::from_partial_motions(
                    board.body_motion.clone().unwrap(),
                    types::motion_command::HeadMotion::Center {
                        image_region_target: types::motion_command::ImageRegion::Center,
                    },
                ));
            if let Some(types::motion_command::MotionCommand::Kick { quick, .. }) =
                &mut board.last_motion_command
            {
                *quick = true;
            }
            board.ball.as_mut().unwrap().position = point![0.2, 0.0];
            select_ball_action(&mut board);
            execute_ball_action(&mut board);
            assert!(matches!(board.ball_action, Some(BallAction::Pass { .. })));
            assert!(matches!(
                board.body_motion,
                Some(BodyMotion::Kick {
                    quick: false,
                    strong: false,
                    ..
                })
            ));

            let mut crowded = board.clone();
            let closer = point![0.9, 0.0];
            crowded
                .world_state
                .player_states
                .two
                .as_mut()
                .unwrap()
                .inner
                .pose = closer.into();
            crowded.world_state.obstacles[0].position = transform.inverse() * closer;
            select_ball_action(&mut crowded);
            execute_ball_action(&mut crowded);
            assert!(
                !matches!(crowded.ball_action, Some(BallAction::Pass { .. })),
                "announced pass must not preserve a point-blank target before launch"
            );

            board.ball.as_mut().unwrap().position = Point2::origin();
            board.last_motion_type = None;

            // A changed target is announced promptly, without resending unchanged intent each tick.
            let receiver = point![2.0, 0.8];
            let state = board.world_state.player_states.two.as_mut().unwrap();
            state.inner.pose = receiver.into();
            state.time = board.world_state.now;
            board.world_state.obstacles[0].position = transform.inverse() * receiver;
            execute_ball_action(&mut board);
            assert_eq!(board.body_motion, Some(BodyMotion::Stand));
            assert!(board.try_sending_state_message().is_some());
            if sub_state.is_some() {
                board.world_state.now = board.world_state.now + Duration::from_millis(150);
            }
            execute_ball_action(&mut board);
            assert!(matches!(board.body_motion, Some(BodyMotion::Kick { .. })));

            board.ball_action = Some(BallAction::Dribble);
            board.world_state.now = board.world_state.now + Duration::from_millis(1);
            let Some(types::messages::OutgoingMessage::Hsl(
                hsl_network_messages::HulkMessage::State(message),
            )) = board.try_sending_state_message()
            else {
                panic!("expected cancellation");
            };
            assert_eq!(message.pass, None);
            assert!(board.last_announced_pass.is_none());
            board
                .world_state
                .filtered_game_controller_state
                .as_mut()
                .unwrap()
                .remaining_number_of_messages = 19;
            board.ball_action = Some(BallAction::Pass {
                receiver: PlayerNumber::Two,
            });
            assert!(board.try_sending_state_message().is_none());
        }
    }

    #[test]
    fn receiver_prepares_for_stationary_ball_then_claims_arrival_and_expires_intent() {
        let mut receiver = passing_blackboard();
        receiver.player_number = PlayerNumber::Two;
        receiver.parameters.goalkeeper.player_number = PlayerNumber::One;
        receiver.world_state.robot.ground_to_field = Some(linear_algebra::Isometry2::from_parts(
            linear_algebra::vector![2.0, 0.0],
            0.0,
        ));
        receiver.world_state.obstacles.clear();
        receiver.world_state.player_states = Default::default();
        receiver.world_state.player_states.three = Some(TimeWrapper {
            time: receiver.world_state.now,
            inner: PlayerState {
                pass: Some(hsl_network_messages::PassIntent {
                    receiver: PlayerNumber::Two,
                    target: point![1.5, 0.0],
                }),
                ..crate::test_support::player_state()
            },
        });
        receiver.world_state.fall_detection = Some(FallDetection {
            time: receiver.world_state.now,
            valid_until: receiver.world_state.now + Duration::from_secs(10),
            posture: Posture::Upright,
        });
        receiver.walking_velocity_limits.angular_velocity_limit = 1.0;
        receiver.parameters.voronoi.grid_resolution = 0.1;
        receiver.parameters.ball.closest_to_ball.enter_duration = Duration::from_millis(300);
        receiver.parameters.ball.last_ball_timeout = Duration::from_secs(1);
        receiver.world_state.ball = Some(BallState {
            ball_in_ground: point![-2.0, 0.0],
            ..types::world_state::BallState::new_at_center(Default::default())
        });
        let tree = create_tree();
        assert!(is_pass_receiver(&mut receiver));
        assert!(!is_pass_ball_arriving(&mut receiver));
        // An 8 m pass at 3.4 m/s can still be in flight well after one second.
        receiver.world_state.now = receiver.world_state.now + Duration::from_millis(2500);
        receiver.ball.as_mut().unwrap().age = receiver.world_state.now;
        assert!(is_pass_receiver(&mut receiver));
        tree.tick_with_trace(&mut receiver);
        let Some(BodyMotion::WalkWithVelocity {
            velocity,
            angular_velocity,
        }) = &receiver.body_motion
        else {
            panic!("receiver must face ball before it moves")
        };
        assert_eq!(velocity.norm(), 0.0);
        assert_eq!(angular_velocity.abs(), 1.0);
        assert!(receiver.kick_target.is_none());

        // The receiver has completed the turn toward the passer.
        receiver.world_state.robot.ground_to_field = Some(linear_algebra::Isometry2::from_parts(
            linear_algebra::vector![2.0, 0.0],
            std::f32::consts::PI,
        ));
        // A nearby opponent requires release rather than the unpressured rolling-ball block.
        receiver.parameters.kicking.receiving_opponent_distance = 2.0;
        receiver.world_state.obstacles = vec![Obstacle::robot(point![1.6, -1.5], 0.2, 0.2)];
        receiver.ball.as_mut().unwrap().position = point![0.4, 0.0];
        receiver.ball.as_mut().unwrap().velocity = linear_algebra::vector![2.0, 0.0];
        receiver.world_state.ball.as_mut().unwrap().ball_in_ground = point![1.6, 0.0];
        receiver.world_state.ball.as_mut().unwrap().ball_in_field = point![0.4, 0.0];
        receiver
            .world_state
            .ball
            .as_mut()
            .unwrap()
            .ball_in_ground_velocity = linear_algebra::vector![-2.0, 0.0];
        assert!(!is_pass_ball_arriving(&mut receiver));
        receiver.parameters.ball.pursuit_lookahead = Duration::from_millis(500);
        tree.tick_with_trace(&mut receiver);
        assert!(receiver.last_closest_to_ball);
        assert!(matches!(
            receiver.body_motion,
            Some(BodyMotion::Kick { .. })
        ));

        // A fast ball beside the receiver must not project beyond the reception area.
        receiver.ball.as_mut().unwrap().position = point![1.6, 0.0];
        receiver.ball.as_mut().unwrap().velocity = linear_algebra::vector![4.0, 0.0];
        let ball = receiver.world_state.ball.as_mut().unwrap();
        ball.ball_in_ground = point![0.4, 0.0];
        ball.ball_in_field = point![1.6, 0.0];
        ball.ball_in_ground_velocity = linear_algebra::vector![-4.0, 0.0];
        tree.tick_with_trace(&mut receiver);
        assert!(
            matches!(receiver.body_motion, Some(BodyMotion::Kick { ball_position, .. })
            if (ball_position - point![0.4, 0.0]).norm() < 1e-6)
        );

        receiver
            .world_state
            .player_states
            .three
            .as_mut()
            .unwrap()
            .inner
            .pass = None;
        assert!(!is_pass_receiver(&mut receiver));
        receiver.kick_target = None;
        tree.tick_with_trace(&mut receiver);
        assert!(matches!(
            receiver.body_motion,
            Some(BodyMotion::Kick { .. })
        ));
        receiver
            .world_state
            .player_states
            .three
            .as_mut()
            .unwrap()
            .inner
            .pass = Some(hsl_network_messages::PassIntent {
            receiver: PlayerNumber::Two,
            target: point![1.5, 0.0],
        });
        receiver.world_state.now = receiver
            .world_state
            .player_states
            .three
            .as_ref()
            .unwrap()
            .time
            + hsl_network_messages::PassIntent::TIMEOUT;
        assert!(!is_pass_receiver(&mut receiver));
    }

    #[test]
    fn pursuit_projection_is_optional_bounded_and_ignores_invalid_velocity() {
        let mut blackboard = blackboard();
        let position = point![1.0, 0.5];
        let velocity = linear_algebra::vector![2.0, -1.0];
        assert_eq!(
            blackboard.projected_ball_position(position, velocity),
            position
        );
        blackboard.parameters.ball.pursuit_lookahead = Duration::from_millis(500);
        assert_eq!(
            blackboard.projected_ball_position(position, velocity),
            point![2.0, 0.0]
        );
        assert_eq!(
            blackboard.projected_ball_position(position, linear_algebra::Vector2::zeros()),
            position
        );
        assert_eq!(
            blackboard.projected_ball_position(position, linear_algebra::vector![f32::NAN, 1.0]),
            position
        );
        let projected =
            blackboard.projected_ball_position(position, linear_algebra::vector![20.0, -20.0]);
        assert!((projected - position).norm() <= 2.0 + 1e-6);
        let outside = point![4.6, 0.0];
        assert_eq!(
            blackboard.projected_ball_position(outside, -velocity),
            outside
        );
        blackboard.world_state.robot.ground_to_field = Some(linear_algebra::Isometry2::from_parts(
            linear_algebra::vector![0.5, 0.5],
            1.0,
        ));
        assert_eq!(
            blackboard.projected_ball_position(position, velocity),
            position
        );
    }

    #[test]
    fn pass_retains_receiver_until_improvement_or_corridor_closure() {
        let mut blackboard = passing_blackboard();
        blackboard
            .world_state
            .player_states
            .two
            .as_mut()
            .unwrap()
            .inner
            .pose = Pose2::new(point![2.0, 1.0], 0.0);
        let fourth = TimeWrapper {
            time: blackboard.world_state.now,
            inner: PlayerState {
                pose: Pose2::new(point![1.9, -1.0], 0.0),
                ..crate::test_support::player_state()
            },
        };
        blackboard.world_state.player_states.four = Some(fourth);
        blackboard.world_state.obstacles.clear();
        for (x, expected) in [
            (1.9, PlayerNumber::Two),
            (2.1, PlayerNumber::Two),
            (2.4, PlayerNumber::Four),
        ] {
            blackboard
                .world_state
                .player_states
                .four
                .as_mut()
                .unwrap()
                .inner
                .pose = Pose2::new(point![x, -1.0], 0.0);
            select_ball_action(&mut blackboard);
            assert_eq!(
                blackboard.ball_action,
                Some(BallAction::Pass { receiver: expected })
            );
        }
        blackboard
            .world_state
            .obstacles
            .push(Obstacle::robot(point![0.8, -0.4], 0.2, 0.2));
        select_ball_action(&mut blackboard);
        assert_eq!(
            blackboard.ball_action,
            Some(BallAction::Pass {
                receiver: PlayerNumber::Two
            })
        );
    }

    #[test]
    fn shots_and_emergency_clears_take_priority_over_passes() {
        let mut blackboard = passing_blackboard();
        blackboard.ball.as_mut().unwrap().position = point![2.0, 0.0];
        blackboard
            .world_state
            .player_states
            .two
            .as_mut()
            .unwrap()
            .inner
            .pose = Pose2::new(point![3.3, 1.0], 0.0);
        blackboard.world_state.obstacles = vec![Obstacle::robot(point![3.3, 1.0], 0.2, 0.2)];
        select_ball_action(&mut blackboard);
        assert!(matches!(
            blackboard.ball_action,
            Some(BallAction::Shoot { .. })
        ));

        blackboard
            .world_state
            .obstacles
            .push(Obstacle::robot(point![3.6, 0.0], 0.4, 0.4));
        select_ball_action(&mut blackboard);
        assert_eq!(
            blackboard.ball_action,
            Some(BallAction::Pass {
                receiver: PlayerNumber::Two
            })
        );

        blackboard.ball.as_mut().unwrap().position = point![-3.8, 0.0];
        blackboard
            .world_state
            .player_states
            .two
            .as_mut()
            .unwrap()
            .inner
            .pose = Pose2::new(point![-1.8, 0.0], 0.0);
        blackboard.world_state.obstacles = vec![Obstacle::robot(point![-3.8, 0.7], 0.2, 0.2)];
        select_ball_action(&mut blackboard);
        assert_eq!(blackboard.ball_action, Some(BallAction::Clear));
    }

    #[test]
    fn known_teammate_near_own_goal_does_not_trigger_emergency_clear() {
        let mut board = blackboard();
        board.ball.as_mut().unwrap().position = point![-3.8, 0.0];
        board
            .parameters
            .kicking
            .strategy
            .passing
            .maximum_receiver_age = Duration::from_secs(3);
        board.world_state.obstacles = vec![Obstacle::robot(point![-3.8, 0.7], 0.2, 0.2)];
        board.world_state.player_states.two = Some(TimeWrapper {
            time: board.world_state.now,
            inner: PlayerState {
                pose: Pose2::new(point![-3.8, 0.7], 0.0),
                ..crate::test_support::player_state()
            },
        });
        assert_eq!(select_ball_action(&mut board), Status::Success);
        assert_eq!(board.ball_action, Some(BallAction::Dribble));

        board.world_state.player_states.two = None;
        select_ball_action(&mut board);
        assert_eq!(board.ball_action, Some(BallAction::Clear));
    }

    #[test]
    fn selects_intent_from_blackboard_and_clears_it_without_ball() {
        let mut blackboard = blackboard();
        assert_eq!(select_ball_action(&mut blackboard), Status::Success);
        assert!(matches!(
            blackboard.ball_action,
            Some(BallAction::Shoot { .. })
        ));
        blackboard.world_state.obstacles = vec![Obstacle::robot(point![3.0, 0.0], 0.5, 0.5)];
        select_ball_action(&mut blackboard);
        assert_eq!(blackboard.ball_action, Some(BallAction::Dribble));
        blackboard.ball.as_mut().unwrap().position = point![-3.8, 0.0];
        blackboard.world_state.obstacles = vec![Obstacle::robot(point![-3.2, 0.0], 0.2, 0.2)];
        select_ball_action(&mut blackboard);
        assert_eq!(blackboard.ball_action, Some(BallAction::Clear));
        blackboard.ball = None;
        assert_eq!(select_ball_action(&mut blackboard), Status::Failure);
        assert_eq!(blackboard.ball_action, None);
    }

    #[test]
    fn dribble_advances_toward_goal_through_a_clear_corridor() {
        let mut blackboard = blackboard();
        let ball = point![1.0, 1.0];
        blackboard.ball.as_mut().unwrap().position = ball;
        blackboard.parameters.kicking.kick_activation_distance = 10.0;
        select_ball_action(&mut blackboard);
        assert_eq!(blackboard.ball_action, Some(BallAction::Dribble));
        execute_ball_action(&mut blackboard);
        let goal = point![blackboard.field_dimensions.length / 2.0, 0.0];
        let target =
            blackboard.world_state.robot.ground_to_field.unwrap() * blackboard.kick_target.unwrap();
        assert!(target.x() > ball.x() && target.x() < goal.x());
        assert!(LineSegment(ball, target).angle(LineSegment(ball, goal)) < 1e-6);

        let obstacle = Obstacle::robot(point![2.0, 0.714], 0.2, 0.2);
        blackboard.world_state.obstacles = vec![obstacle];
        execute_ball_action(&mut blackboard);
        let target =
            blackboard.world_state.robot.ground_to_field.unwrap() * blackboard.kick_target.unwrap();
        assert!(
            LineSegment(ball, target).distance_to(
                blackboard.world_state.robot.ground_to_field.unwrap() * obstacle.position,
            ) > obstacle.radius_at_foot_height
                + blackboard.field_dimensions.ball_radius
                + blackboard.parameters.kicking.strategy.obstacle_clearance
        );
        // Detour just around the blocker instead of aiming at the opening's midpoint.
        assert!(LineSegment(ball, target).angle(LineSegment(ball, goal)) < 0.4);

        // Strategy and execution must use the same observed ball, not disagree on range.
        blackboard.world_state.obstacles.clear();
        blackboard.world_state.ball = Some(BallState {
            ball_in_ground: point![2.0, 0.0],
            ball_in_field: point![2.0, 0.0],
            ..BallState::new_at_center(Default::default())
        });
        select_ball_action(&mut blackboard);
        assert!(matches!(
            blackboard.ball_action,
            Some(BallAction::Shoot { .. })
        ));
        execute_ball_action(&mut blackboard);
        assert_eq!(blackboard.kick_target.unwrap().x(), goal.x());
    }

    #[test]
    fn dribble_recovers_from_h2_end_line_position_without_shortening_travel() {
        let mut blackboard = blackboard();
        blackboard.field_dimensions = FieldDimensions {
            length: 8.95,
            width: 5.95,
            goal_inner_width: 2.4,
            ball_radius: 0.105,
            line_width: 0.06,
            ..FieldDimensions::SPL_2025
        };
        let field = blackboard.field_dimensions;
        let ball = point![4.219_695, -2.053_933_6];
        let transform =
            linear_algebra::Isometry2::from_parts(linear_algebra::vector![3.99, -1.86], -0.43);
        blackboard.world_state.robot.ground_to_field = Some(transform);
        blackboard.ball.as_mut().unwrap().position = ball;
        blackboard.parameters.kicking.kick_activation_distance = 3.0;
        blackboard.world_state.obstacles = [Half::Own, Half::Opponent]
            .into_iter()
            .flat_map(|half| {
                [Side::Left, Side::Right].map(move |side| {
                    Obstacle::goal_post(
                        transform.inverse() * field.goal_post(half, side),
                        field.goal_post_diameter / 2.0,
                    )
                })
            })
            .chain(
                [
                    point![4.278_642_7, -1.109_891_7],
                    point![-4.045, -0.018],
                    point![-1.616, 0.0],
                ]
                .map(|position| Obstacle::robot(transform.inverse() * position, 0.2, 0.25)),
            )
            .collect();

        select_ball_action(&mut blackboard);
        assert_eq!(blackboard.ball_action, Some(BallAction::Dribble));
        execute_ball_action(&mut blackboard);
        let target = transform * blackboard.kick_target.unwrap();
        let travel = blackboard
            .parameters
            .kicking
            .strategy
            .maximum_dribble_distance;
        let margin = field.ball_radius + blackboard.parameters.kicking.strategy.obstacle_clearance;
        assert!((target - ball).norm() >= travel - 1e-5);
        assert!(target.x() < ball.x() && target.y() > ball.y());
        assert!(target.x().abs() <= field.length / 2.0 - margin);
        assert!(target.y().abs() <= field.width / 2.0 - margin);
        for obstacle in &blackboard.world_state.obstacles {
            assert!(
                LineSegment(ball, target).distance_to(transform * obstacle.position)
                    > obstacle.radius_at_foot_height + margin
            );
        }
        assert!(matches!(
            blackboard.body_motion,
            Some(BodyMotion::Kick {
                soft: false,
                strong: false,
                ..
            })
        ));
    }

    #[test]
    fn unavailable_dribble_reselects_valid_shot_but_invalid_parameters_stop() {
        let mut blackboard = blackboard();
        blackboard.ball.as_mut().unwrap().position = point![0.0, 0.0];
        blackboard.field_dimensions.length = 1.0;
        blackboard.field_dimensions.width = 1.0;
        blackboard.parameters.kicking.kick_activation_distance = 1.0;
        for distance in [3.0, 0.0, -1.0, f32::NAN, f32::INFINITY] {
            blackboard.ball_action = Some(BallAction::Dribble);
            blackboard
                .parameters
                .kicking
                .strategy
                .maximum_dribble_distance = distance;
            execute_ball_action(&mut blackboard);
            if distance.is_finite() && distance >= 0.0 {
                assert!(matches!(
                    blackboard.body_motion,
                    Some(BodyMotion::Kick { .. })
                ));
                assert!((blackboard.kick_target.unwrap() - point![0.5, 0.0]).norm() < 1e-6);
                assert!(matches!(
                    blackboard.ball_action,
                    Some(BallAction::Shoot { .. })
                ));
            } else {
                assert_eq!(blackboard.body_motion, Some(BodyMotion::Stand));
                assert_eq!(blackboard.kick_target, None);
                assert_eq!(blackboard.ball_action, None);
            }
        }
    }

    #[test]
    fn invalid_strategy_parameters_stop_selection_and_execution() {
        for invalid in [-0.01, f32::NAN, f32::INFINITY] {
            let mut blackboard = blackboard();
            blackboard.parameters.kicking.strategy.shot_angle_hysteresis = invalid;
            assert_eq!(select_ball_action(&mut blackboard), Status::Failure);
            assert_eq!(blackboard.ball_action, None);
            blackboard.ball_action = Some(BallAction::Dribble);
            execute_ball_action(&mut blackboard);
            assert_eq!(blackboard.body_motion, Some(BodyMotion::Stand));
            assert_eq!(blackboard.kick_target, None);
        }
    }

    #[test]
    fn walking_fallback_retains_ktsl_pass_or_dribble_and_revalidates_corridors() {
        let tree = crate::walk::walk_alternatives_subtree();
        for action in [
            BallAction::Pass {
                receiver: PlayerNumber::Two,
            },
            BallAction::Dribble,
        ] {
            let mut board = passing_blackboard();
            board.ball_action = Some(action);
            if action == BallAction::Dribble {
                board.world_state.player_states.two = None;
                board.world_state.obstacles.clear();
            }
            execute_ball_action(&mut board);
            if matches!(action, BallAction::Pass { .. }) {
                assert!(board.try_sending_state_message().is_some());
                board.world_state.now = board.world_state.now + Duration::from_millis(150);
                execute_ball_action(&mut board);
            }
            let target = board.kick_target;
            board.last_motion_type = Some(types::motion_type::MotionType::Kick);
            board.last_motion_command =
                Some(types::motion_command::MotionCommand::from_partial_motions(
                    board.body_motion.clone().unwrap(),
                    types::motion_command::HeadMotion::LookAround,
                ));
            board.kick_target = None;
            board.body_motion = None;
            tree.tick_with_trace(&mut board);
            assert_eq!(board.ball_action, Some(action));
            assert_eq!(board.kick_target, target);
            assert!(matches!(
                board.body_motion,
                Some(BodyMotion::Kick {
                    strong: false,
                    soft: false,
                    ..
                })
            ));

            // A newly occupied ball corridor cancels continuation instead of aiming at goal.
            board.world_state.obstacles = vec![Obstacle::robot(point![0.0, 0.0], 0.4, 0.4)];
            board.kick_target = None;
            board.body_motion = None;
            tree.tick_with_trace(&mut board);
            assert_eq!(board.ball_action, None);
            assert_eq!(board.kick_target, None);
            assert_eq!(board.body_motion, Some(BodyMotion::Stand));
        }
    }

    #[test]
    fn goal_center_fallback_stops_without_valid_ball_or_localization() {
        for case in 0..5 {
            let mut blackboard = blackboard();
            blackboard.ball_action = Some(BallAction::ShootAtGoalCenter);
            match case {
                0 => blackboard.ball = None,
                1 => blackboard.world_state.robot.ground_to_field = None,
                2 => blackboard.ball.as_mut().unwrap().position = point![4.6, 0.0],
                3 => blackboard.ball.as_mut().unwrap().position = point![0.0, 3.1],
                _ => blackboard.ball.as_mut().unwrap().position = point![f32::NAN, 0.0],
            }
            execute_ball_action(&mut blackboard);
            assert_eq!(
                blackboard.body_motion,
                Some(BodyMotion::Stand),
                "case {case}"
            );
            assert_eq!(blackboard.kick_target, None, "case {case}");
        }
    }

    #[test]
    fn releases_ball_under_pressure_when_no_shot_or_pass_is_available() {
        let mut board = blackboard();
        board.ball.as_mut().unwrap().position = point![0.0, 0.0];
        board.world_state.obstacles = vec![Obstacle::robot(point![0.5, 0.4], 0.2, 0.2)];
        assert_eq!(select_ball_action(&mut board), Status::Success);
        assert_eq!(board.ball_action, Some(BallAction::Clear));
        let target = crate::kick::execution::resolve_target(&board).unwrap();
        assert!(target.x() > 0.0);

        board.world_state.obstacles.clear();
        assert_eq!(select_ball_action(&mut board), Status::Success);
        assert_eq!(board.ball_action, Some(BallAction::Dribble));
    }

    #[test]
    fn own_goal_kicks_choose_deliberate_passes_instead_of_emergency_clearances() {
        let mut board = passing_blackboard();
        board.ball.as_mut().unwrap().position = point![-3.8, 0.0];
        board
            .world_state
            .player_states
            .two
            .as_mut()
            .unwrap()
            .inner
            .pose = point![-1.5, 0.0].into();
        let game = board
            .world_state
            .filtered_game_controller_state
            .as_mut()
            .unwrap();
        game.kicking_team = Some(Team::Hulks);
        game.sub_state = Some(SubState::GoalKick);
        for opponent in [point![0.0, 2.0], point![-3.8, 0.7]] {
            board.world_state.obstacles = vec![Obstacle::robot(opponent, 0.2, 0.2)];
            board.ball_action = Some(BallAction::Clear);
            select_ball_action(&mut board);
            assert_eq!(
                board.ball_action,
                Some(BallAction::Pass {
                    receiver: PlayerNumber::Two
                })
            );
        }
        board
            .world_state
            .filtered_game_controller_state
            .as_mut()
            .unwrap()
            .kicking_team = Some(Team::Opponent);
        select_ball_action(&mut board);
        assert_eq!(board.ball_action, Some(BallAction::Clear));
    }

    #[test]
    fn pressured_clearances_choose_goalward_diagonals_over_wide_sideways_gaps() {
        for side in [-1.0, 1.0] {
            let mut board = blackboard();
            board.field_dimensions = FieldDimensions {
                length: 22.0,
                width: 14.0,
                goal_inner_width: 3.0,
                ball_radius: 0.11,
                ..FieldDimensions::SPL_2025
            };
            let ball = point![-2.0, side * 5.8];
            let transform = Isometry2::from_parts(vector![-2.6, ball.y()], 0.0);
            let opponent = point![-1.3, side * 5.5];
            board.world_state.robot.ground_to_field = Some(transform);
            board.ball.as_mut().unwrap().position = ball;
            board.world_state.obstacles =
                vec![Obstacle::robot(transform.inverse() * opponent, 0.2, 0.2)];
            board.parameters.kicking.strategy.maximum_shot_distance = 12.0;
            board.parameters.kicking.kick_activation_distance = 1.3;
            board.parameters.kicking.allow_strong_kicks = true;

            assert_eq!(select_ball_action(&mut board), Status::Success);
            assert_eq!(board.ball_action, Some(BallAction::Clear));
            assert_eq!(execute_ball_action(&mut board), Status::Success);
            let target = transform * board.kick_target.unwrap();
            let direction = (target - ball).normalize();
            assert!(direction.x() > 0.0 && direction.y() * side < 0.0);
            assert!(direction.y().atan2(direction.x()).abs() < 55.0_f32.to_radians());
            assert!(target.x().abs() < 11.0 && target.y().abs() < 7.0);
            assert!(LineSegment(ball, target).distance_to(opponent) > 0.36);
            assert!(matches!(
                board.body_motion,
                Some(BodyMotion::Kick {
                    quick: true,
                    strong: true,
                    soft: false,
                    ..
                })
            ));
        }
    }

    #[test]
    fn clear_uses_forward_part_of_a_sideways_opening() {
        let mut blackboard = blackboard();
        let ball = point![-3.8, -0.8];
        blackboard.ball.as_mut().unwrap().position = ball;
        blackboard.parameters.kicking.kick_activation_distance = 5.0;
        blackboard.world_state.obstacles = [
            point![-3.123_852, -0.981_173],
            point![-3.193_782, -0.45],
            point![-3.45, -0.193_782],
        ]
        .map(|position| Obstacle::robot(position, 0.2, 0.2))
        .to_vec();
        select_ball_action(&mut blackboard);
        assert_eq!(blackboard.ball_action, Some(BallAction::Clear));
        execute_ball_action(&mut blackboard);
        assert!(matches!(
            blackboard.body_motion,
            Some(BodyMotion::Walk { .. } | BodyMotion::WalkWithVelocity { .. })
        ));
        let transform = blackboard.world_state.robot.ground_to_field.unwrap();
        let target = transform * blackboard.kick_target.unwrap();
        assert!(target.x() > ball.x());
        assert!((target - ball).norm() >= 3.0);
        for obstacle in &blackboard.world_state.obstacles {
            assert!(
                LineSegment(ball, target).distance_to(transform * obstacle.position)
                    > obstacle.radius_at_foot_height
                        + blackboard.field_dimensions.ball_radius
                        + blackboard.parameters.kicking.strategy.obstacle_clearance
            );
        }
    }

    #[test]
    fn kick_handoff_uses_coherent_world_ball_and_cached_fallback() {
        let mut blackboard = blackboard();
        blackboard.parameters.kicking.kick_activation_distance = 1.0;
        blackboard.visual_kick_ball_position = Some(BallPosition {
            position: point![0.9, 0.0],
            velocity: Default::default(),
            last_seen: Default::default(),
        });
        blackboard.world_state.ball = Some(BallState {
            ball_in_ground: point![0.5, 0.0],
            ball_in_field: point![0.5, 0.0],
            ball_in_ground_velocity: linear_algebra::vector![0.4, 0.1],
            ..BallState::new_at_center(Default::default())
        });
        select_ball_action(&mut blackboard);
        execute_ball_action(&mut blackboard);
        assert!(matches!(
            blackboard.body_motion,
            Some(BodyMotion::Kick { ball_position, ball_velocity, .. })
                if ball_position == point![0.5, 0.0] && ball_velocity == linear_algebra::vector![0.4, 0.1]
        ));

        blackboard.ball.as_mut().unwrap().position = point![0.5, 0.0];
        blackboard.world_state.ball = None;
        blackboard
            .visual_kick_ball_position
            .as_mut()
            .unwrap()
            .position = point![2.0, 0.0];
        select_ball_action(&mut blackboard);
        execute_ball_action(&mut blackboard);
        assert!(matches!(
            blackboard.body_motion,
            Some(BodyMotion::Kick { ball_position, .. }) if ball_position == point![0.5, 0.0]
        ));
    }

    #[test]
    fn retains_shot_side_under_small_changes_but_rejects_blocked_opening() {
        let mut blackboard = blackboard();
        blackboard.world_state.obstacles = vec![Obstacle::robot(point![3.5, -0.02], 0.1, 0.1)];
        for y in [-0.02, 0.02] {
            blackboard.world_state.obstacles[0].position = point![3.5, y];
            select_ball_action(&mut blackboard);
            let Some(BallAction::Shoot { opening }) = blackboard.ball_action else {
                panic!("expected shot")
            };
            assert!(opening.center().y() > 0.0);
        }
        blackboard.world_state.obstacles[0].radius_at_foot_height = 1.0;
        select_ball_action(&mut blackboard);
        assert_eq!(blackboard.ball_action, Some(BallAction::Dribble));
    }

    #[test]
    fn execution_turns_before_approaching_a_stationary_ball_behind_the_body() {
        let mut blackboard = blackboard();
        blackboard.ball.as_mut().unwrap().position = point![-0.55, 0.2];
        blackboard.world_state.ball = Some(BallState {
            ball_in_ground: point![-0.55, 0.2],
            ball_in_field: point![-0.55, 0.2],
            ..types::world_state::BallState::new_at_center(Default::default())
        });
        blackboard.parameters.kicking.kick_activation_distance = 1.0;
        blackboard.parameters.kicking.approach_ball_standoff = 0.3;
        blackboard.walking_velocity_limits.angular_velocity_limit = 2.0;
        select_ball_action(&mut blackboard);
        assert_eq!(execute_ball_action(&mut blackboard), Status::Success);
        assert!(matches!(
            blackboard.body_motion,
            Some(BodyMotion::WalkWithVelocity { velocity, angular_velocity })
                if velocity == linear_algebra::Vector2::zeros() && angular_velocity > 0.0
        ));
    }

    #[test]
    fn execution_approaches_ball_revalidates_and_kicks_to_advance() {
        let mut blackboard = blackboard();
        blackboard.parameters.kicking.kick_activation_distance = 1.0;
        blackboard.parameters.kicking.approach_ball_standoff = 0.3;
        blackboard
            .parameters
            .walking
            .walk_and_stand
            .target_reached_thresholds = nalgebra::vector![0.05, 0.1];
        blackboard.parameters.walking.path_planning.robot_radius = 0.4;
        blackboard
            .parameters
            .walking
            .path_planning
            .ball_obstacle_radius = 0.05;
        let transform =
            linear_algebra::Isometry2::from_parts(linear_algebra::vector![0.2, -0.1], 0.3);
        blackboard.world_state.robot.ground_to_field = Some(transform);
        blackboard.parameters.ball.pursuit_lookahead = Duration::from_millis(500);
        blackboard.ball.as_mut().unwrap().velocity = linear_algebra::vector![0.0, 1.0];
        let observed = transform.inverse() * point![2.0, 0.0];
        let velocity = transform.inverse() * linear_algebra::vector![0.0, 1.0];
        blackboard.world_state.ball = Some(BallState {
            ball_in_ground: observed,
            ball_in_field: point![2.0, 0.0],
            ball_in_ground_velocity: velocity,
            ..types::world_state::BallState::new_at_center(Default::default())
        });
        blackboard.world_state.obstacles = vec![Obstacle::robot(
            transform.inverse() * point![3.5, 0.0],
            0.1,
            0.1,
        )];
        let tree = striker_subtree();
        assert_eq!(tree.tick_with_trace(&mut blackboard).0, Status::Success);
        let target = transform * blackboard.kick_target.unwrap();
        let Some(BodyMotion::Walk {
            target_orientation,
            ref path,
            ..
        }) = blackboard.body_motion
        else {
            panic!("expected approach")
        };
        let ball_in_ground = transform.inverse() * point![2.0, 0.5];
        let facing_ball = linear_algebra::Orientation2::from_vector(observed.coords());
        assert!((target_orientation.angle() - facing_ball.angle()).abs() < 1e-6);
        let expected_approach = ball_in_ground - ball_in_ground.coords().normalize() * 0.3;
        let types::path::PathSegment::LineSegment(last) = path.last_segment() else {
            panic!("expected final straight approach")
        };
        assert!((last.1 - expected_approach).norm() < 1e-5);
        let expected = linear_algebra::Orientation2::from_vector(
            transform.inverse() * (target - point![2.0, 0.0]),
        );
        blackboard.parameters.kicking.kick_activation_distance = 3.0;
        tree.tick_with_trace(&mut blackboard);
        let Some(BodyMotion::Kick {
            kick_direction,
            ball_position,
            ball_velocity,
            ..
        }) = blackboard.body_motion
        else {
            panic!("expected shot")
        };
        assert!((blackboard.kick_target.unwrap() - transform.inverse() * target).norm() < 1e-6);
        assert!((kick_direction.angle() - expected.angle()).abs() < 1e-6);
        assert_eq!(ball_position, observed);
        assert_eq!(ball_velocity, velocity);
        blackboard.world_state.obstacles[0].radius_at_foot_height = 1.0;
        execute_ball_action(&mut blackboard);
        assert!(matches!(
            blackboard.body_motion,
            Some(BodyMotion::Kick {
                strong: false,
                soft: false,
                ..
            })
        ));
        assert_eq!(blackboard.ball_action, Some(BallAction::Dribble));
        let target = transform * blackboard.kick_target.unwrap();
        let blocker = blackboard.world_state.obstacles[0];
        assert!(
            LineSegment(point![2.0, 0.0], target).distance_to(transform * blocker.position)
                > blocker.radius_at_foot_height + 0.1
        );

        let transform = Pose2::new(point![-4.2, 0.0], 0.0).as_transform();
        blackboard.world_state.robot.ground_to_field = Some(transform);
        blackboard.ball.as_mut().unwrap().position = point![-3.8, 0.0];
        blackboard.ball.as_mut().unwrap().velocity = Default::default();
        blackboard.world_state.ball = Some(BallState {
            ball_in_ground: transform.inverse() * point![-3.8, 0.0],
            ball_in_field: point![-3.8, 0.0],
            ..types::world_state::BallState::new_at_center(Default::default())
        });
        blackboard.world_state.obstacles = vec![Obstacle::robot(
            transform.inverse() * point![-3.8, 0.7],
            0.2,
            0.2,
        )];
        blackboard.parameters.kicking.kick_activation_distance = 5.0;
        tree.tick_with_trace(&mut blackboard);
        assert_eq!(blackboard.ball_action, Some(BallAction::Clear));
        assert!((transform * blackboard.kick_target.unwrap()).x() > -3.8);
        assert!(matches!(
            blackboard.body_motion,
            Some(BodyMotion::Kick { .. })
        ));

        // The recorded stall had 6 cm lateral error; kick-policy alignment must handle it.
        blackboard.world_state.robot.ground_to_field = Some(Default::default());
        blackboard.parameters.kicking.kick_activation_distance = 2.0;
        blackboard.ball.as_mut().unwrap().position = point![0.51, -0.06];
        blackboard.world_state.ball = Some(BallState {
            ball_in_ground: point![0.51, -0.06],
            ball_in_field: point![0.51, -0.06],
            ..BallState::new_at_center(Default::default())
        });
        blackboard.world_state.obstacles.clear();
        blackboard.parameters.kicking.allow_strong_kicks = true;
        blackboard
            .parameters
            .kicking
            .strong_kick_min_target_distance = 0.1;
        blackboard.last_motion_command =
            Some(types::motion_command::MotionCommand::from_partial_motions(
                BodyMotion::Kick {
                    ball_position: point![0.51, -0.06],
                    ball_velocity: Default::default(),
                    target_speed: 1.0,
                    soft: false,
                    quick: false,
                    kick_direction: Default::default(),
                    strong: true,
                },
                types::motion_command::HeadMotion::Center {
                    image_region_target: types::motion_command::ImageRegion::Center,
                },
            ));
        tree.tick_with_trace(&mut blackboard);
        assert_eq!(blackboard.ball_action, Some(BallAction::Dribble));
        assert!(matches!(
            blackboard.body_motion,
            Some(BodyMotion::Kick {
                strong: false,
                soft: false,
                ..
            })
        ));
        assert!(blackboard.kick_target.unwrap().x() > 0.51);
        blackboard.world_state.obstacles = vec![Obstacle::robot(point![0.6, 0.0], 0.3, 0.3)];
        // The node resets per-tick outputs before running the tree.
        blackboard.kick_target = None;
        blackboard.body_motion = None;
        tree.tick_with_trace(&mut blackboard);
        assert_eq!(blackboard.body_motion, Some(BodyMotion::Stand));
        assert_eq!(blackboard.ball_action, None);
        assert_eq!(blackboard.kick_target, None);
    }
}
