use coordinate_systems::{Field, Ground};
use filtering::hysteresis::{
    greater_than_with_relative_hysteresis, less_than_with_relative_hysteresis,
};
use geometry::{Distance, line_segment::LineSegment};
use hsl_network_messages::{PlayerNumber, SubState, Team};
use linear_algebra::{Isometry2, Point2, point};
use ros_z::Message;
use serde::{Deserialize, Serialize};
use types::{
    behavior_tree::Status,
    field_dimensions::Half,
    motion_type::MotionType,
    obstacles::{Obstacle, ObstacleKind},
};

use crate::node::Blackboard;

use super::{
    execution::travel_target,
    geometry::{BALL_FIELD_BOUNDARY_TOLERANCE, goal_mouth, goal_openings, pass_openings},
    kick_ball_position_in_ground,
    passing::{is_executing_announced_pass, pass_receiver_position},
};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Message)]
pub enum QuickKickReason {
    NearOwnGoal,
    NearOpponentGoal,
    NearbyRobot,
    NoUsablePlan,
}

/// Tactical intent only. Execution chooses the target, approach, and physical motion.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Message)]
pub enum BallAction {
    /// Release goalward immediately; execution still checks reach and motion switching.
    QuickKick {
        reason: QuickKickReason,
    },
    Shoot {
        opening: LineSegment<Field>,
    },
    Pass {
        receiver: PlayerNumber,
    },
    Dribble,
    Clear,
}

pub fn begin_ball_action_cycle(blackboard: &mut Blackboard) -> Status {
    blackboard.last_ball_action = blackboard.ball_action.take();
    Status::Success
}

pub fn is_quick_kick_selected(blackboard: &mut Blackboard) -> bool {
    matches!(blackboard.ball_action, Some(BallAction::QuickKick { .. }))
}

/// Select ordinary striker intent. Clear `ball_action` when leaving that context.
pub fn select_ball_action(blackboard: &mut Blackboard) -> Status {
    blackboard.ball_action = choose_ball_action(blackboard);
    if blackboard.ball_action.is_some() {
        Status::Success
    } else {
        Status::Failure
    }
}

fn choose_ball_action(blackboard: &Blackboard) -> Option<BallAction> {
    let ball = blackboard.ball.as_ref()?.position;
    let world = &blackboard.world_state;
    let field = &blackboard.field_dimensions;
    let parameters = &blackboard.parameters.kicking.strategy;
    parameters.validate().ok()?;
    let dribble_speed = blackboard.parameters.kicking.dribble_target_speed;
    if !dribble_speed.is_finite() || dribble_speed < 0.0 {
        return None;
    }
    let previous = blackboard.ball_action.or(blackboard.last_ball_action);
    let ground_to_field = world.robot.ground_to_field?;
    let travel_ball = ground_to_field * kick_ball_position_in_ground(blackboard, &ground_to_field)?;
    let goal_x = field.length / 2.0;
    if !field.length.is_finite()
        || field.length <= 0.0
        || !field.width.is_finite()
        || field.width <= 0.0
    {
        return None;
    }
    if blackboard.last_motion_type == Some(MotionType::Kick)
        && previous.is_some()
        && !matches!(previous, Some(BallAction::Pass { .. }))
    {
        return previous;
    }
    let urgent = urgent_kick_reason(blackboard);
    if ball.x().abs() >= goal_x || ball.y().abs() >= field.width / 2.0 {
        return urgent.map(|reason| BallAction::QuickKick { reason });
    }
    let was_clearing = matches!(previous, Some(BallAction::Clear));
    let own_goal = LineSegment(
        point![-goal_x, -field.goal_inner_width / 2.0],
        point![-goal_x, field.goal_inner_width / 2.0],
    );
    let ball_in_ground = ground_to_field.inverse() * travel_ball;
    // Robot affiliation is unknown; nearby robots conservatively count as pressure.
    let under_pressure = world.obstacles.iter().any(|obstacle| {
        matches!(
            obstacle.kind,
            ObstacleKind::Robot | ObstacleKind::Person | ObstacleKind::Unknown
        ) && less_than_with_relative_hysteresis(
            was_clearing,
            (obstacle.position - ball_in_ground).norm() - obstacle.radius_at_foot_height,
            parameters.pressure_distance,
            0.0..=parameters.clear_distance_hysteresis,
        )
    });
    let can_clear = under_pressure
        && travel_target(blackboard, BallAction::Clear, ground_to_field, travel_ball).is_some();
    if can_clear
        && less_than_with_relative_hysteresis(
            was_clearing,
            own_goal.distance_to(travel_ball),
            parameters.clear_goal_distance,
            0.0..=parameters.clear_distance_hysteresis,
        )
    {
        return Some(BallAction::Clear);
    }

    let range = parameters.maximum_shot_distance;
    if range.is_finite() && range > goal_x - ball.x() {
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
        if let Some((opening, angle)) = best
            && greater_than_with_relative_hysteresis(
                matches!(previous, Some(BallAction::Shoot { .. })),
                angle,
                parameters.minimum_shot_angle,
                0.0..=parameters.shot_angle_hysteresis,
            )
        {
            let opening = retained
                .filter(|(_, old_angle)| angle <= old_angle + parameters.shot_angle_hysteresis)
                .map_or(opening, |(opening, _)| opening);
            return Some(urgent.map_or(BallAction::Shoot { opening }, |reason| {
                BallAction::QuickKick { reason }
            }));
        }
    }

    if can_clear {
        Some(BallAction::Clear)
    } else if travel_target(
        blackboard,
        BallAction::Dribble,
        ground_to_field,
        travel_ball,
    )
    .is_some()
    {
        Some(BallAction::Dribble)
    } else {
        Some(BallAction::QuickKick {
            reason: urgent.unwrap_or(QuickKickReason::NoUsablePlan),
        })
    }
}

pub(super) fn urgent_kick_reason(blackboard: &Blackboard) -> Option<QuickKickReason> {
    let ground_to_field = blackboard.world_state.robot.ground_to_field?;
    let robot = ground_to_field * point![0.0, 0.0];
    // Initial tuning: robot-to-goal-mouth distance, matching the 3 m shot range.
    for (half, reason) in [
        (Half::Own, QuickKickReason::NearOwnGoal),
        (Half::Opponent, QuickKickReason::NearOpponentGoal),
    ] {
        if goal_mouth(&blackboard.field_dimensions, half).distance_to(robot) <= 3.0 {
            return Some(reason);
        }
    }
    // Ground positions are relative to us, not the ball. Use centre distances.
    let nearby_obstacle = blackboard.world_state.obstacles.iter().any(|obstacle| {
        matches!(
            obstacle.kind,
            ObstacleKind::Robot | ObstacleKind::Person | ObstacleKind::Unknown
        ) && obstacle.position.coords().norm() <= 1.5
    });
    let nearby_teammate = blackboard
        .world_state
        .player_states
        .iter()
        .any(|(number, state)| {
            number != blackboard.player_number
                && state
                    .as_ref()
                    .is_some_and(|state| (state.inner.pose.position() - robot).norm() <= 1.5)
        });
    (nearby_obstacle || nearby_teammate).then_some(QuickKickReason::NearbyRobot)
}

pub(crate) fn own_non_penalty_restart(blackboard: &Blackboard) -> bool {
    blackboard
        .world_state
        .filtered_game_controller_state
        .as_ref()
        .is_some_and(|game| {
            game.kicking_team == Some(Team::Hulks)
                && game
                    .sub_state
                    .is_some_and(|state| state != SubState::PenaltyKick)
                && matches!(
                    game.game_state,
                    types::filtered_game_state::FilteredGameState::Playing { .. }
                )
        })
}

/// Select a fresh pass only during our non-penalty restarts.
pub fn select_restart_ball_action(blackboard: &mut Blackboard) -> Status {
    blackboard.ball_action = None;
    if !own_non_penalty_restart(blackboard)
        || blackboard.parameters.kicking.strategy.validate().is_err()
    {
        return Status::Failure;
    }
    let Some(transform) = blackboard.world_state.robot.ground_to_field else {
        return Status::Failure;
    };
    let Some((ball, _)) = super::kick_ball_in_ground(blackboard, &transform) else {
        return Status::Failure;
    };
    let ball = transform * ball;
    let field = &blackboard.field_dimensions;
    if !field.length.is_finite()
        || !field.width.is_finite()
        || field.length <= 0.0
        || field.width <= 0.0
        || ball.x().abs() > field.length / 2.0 + BALL_FIELD_BOUNDARY_TOLERANCE
        || ball.y().abs() > field.width / 2.0 + BALL_FIELD_BOUNDARY_TOLERANCE
    {
        return Status::Failure;
    }
    blackboard.ball_action = choose_pass(blackboard, ball, transform);
    if blackboard.ball_action.is_some() {
        Status::Success
    } else {
        Status::Failure
    }
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
    let previous = blackboard.ball_action.or(blackboard.last_ball_action);
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
    use types::{
        field_dimensions::{FieldDimensions, Side},
        obstacles::Obstacle,
        parameters::KickStrategyParameters,
    };

    use crate::node::LastBall;

    use super::*;

    fn blackboard() -> Blackboard {
        let mut blackboard = Blackboard {
            field_dimensions: FieldDimensions::SPL_2025,
            ball: Some(LastBall {
                position: point![2.0, 0.0],
                velocity: Default::default(),
                age: Default::default(),
                field_side: Side::Left,
                source: Default::default(),
            }),
            parameters: json5::from_str(include_str!(
                "../../../../../etc/parameters/base/behavior_node.json5"
            ))
            .unwrap(),
            player_number: hsl_network_messages::PlayerNumber::One,
            head_yaw: 0.0,
            walking_velocity_limits: Default::default(),
            search_cell_size: None,
            world_state: Default::default(),
            controller_input: None,
            remote_control_enabled: false,
            path_obstacles_output: Vec::new(),
            time_since_last_switch: Default::default(),
            direction_difference: 0.0,
            voronoi_inputs: Vec::new(),
            visual_kick_ball_position: None,
            last_ball: None,
            last_close_enough_to_kick: false,
            ball_action: None,
            last_ball_action: None,
            kick_target: None,
            last_kick_target: None,
            last_motion_command: None,
            last_motion_switch_time: Default::default(),
            last_motion_type: None,
            last_sent_game_controller_return_message_time: None,
            last_sent_hsl_message_time: None,
            last_announced_pass: None,
            kickoff_receiver_wait_started: None,
            restart_pass_wait_started: None,
            kickoff_prepared: false,
            kickoff_started: None,
            last_closest_to_ball: false,
            closest_to_ball_entered_area_since: None,
            closest_to_ball_left_area_since: None,
            is_injected_motion_command: false,
            walk_position: None,
            body_motion: None,
            head_motion: None,
            voronoi_map: None,
        };
        blackboard.world_state.robot.ground_to_field = Some(Default::default());
        blackboard.parameters.kicking.strategy = KickStrategyParameters {
            maximum_shot_distance: 3.0,
            maximum_dribble_distance: 0.0,
            obstacle_clearance: 0.05,
            minimum_shot_angle: 0.02,
            shot_angle_hysteresis: 0.05,
            pressure_distance: 0.8,
            clear_goal_distance: 1.5,
            clear_distance_hysteresis: 0.2,
            passing: Default::default(),
        };
        blackboard
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
        assert_eq!(
            blackboard.ball_action,
            Some(BallAction::QuickKick {
                reason: QuickKickReason::NoUsablePlan
            })
        );
        blackboard.ball.as_mut().unwrap().position = point![-3.8, 0.0];
        blackboard.world_state.obstacles = vec![Obstacle::robot(point![-3.2, 0.0], 0.2, 0.2)];
        select_ball_action(&mut blackboard);
        assert_eq!(blackboard.ball_action, Some(BallAction::Clear));
        blackboard.ball = None;
        assert_eq!(select_ball_action(&mut blackboard), Status::Failure);
        assert_eq!(blackboard.ball_action, None);
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
        assert_eq!(
            blackboard.ball_action,
            Some(BallAction::QuickKick {
                reason: QuickKickReason::NoUsablePlan
            })
        );
    }
    #[test]
    fn ordinary_play_never_selects_a_new_pass_even_with_an_open_receiver() {
        let mut board =
            crate::test_support::restart_blackboard(SubState::DirectFreeKick, point![0.0, 0.0]);
        board.world_state.player_states.two = Some(types::time_wrapper::TimeWrapper {
            time: board.world_state.now,
            inner: types::world_state::PlayerState {
                pose: point![3.0, 0.0].into(),
                ..crate::test_support::player_state()
            },
        });
        assert_eq!(select_restart_ball_action(&mut board), Status::Success);
        assert!(matches!(board.ball_action, Some(BallAction::Pass { .. })));
        board
            .world_state
            .filtered_game_controller_state
            .as_mut()
            .unwrap()
            .sub_state = None;
        let pass = board.ball_action;
        for current in [true, false] {
            for expired in [false, true] {
                let mut ordinary = board.clone();
                ordinary.last_motion_type = Some(MotionType::Kick);
                ordinary.ball_action = if current { pass } else { None };
                ordinary.last_ball_action = pass;
                if expired {
                    ordinary.last_announced_pass = Some(types::time_wrapper::TimeWrapper {
                        time: ordinary.world_state.now - std::time::Duration::from_secs(6),
                        inner: hsl_network_messages::PassIntent {
                            receiver: PlayerNumber::Two,
                            target: point![2.5, 0.0],
                        },
                    });
                }
                select_ball_action(&mut ordinary);
                assert!(!matches!(
                    ordinary.ball_action,
                    Some(BallAction::Pass { .. })
                ));
                assert_eq!(select_restart_ball_action(&mut ordinary), Status::Failure);
                assert!(ordinary.ball_action.is_none());
            }
        }
    }
}
