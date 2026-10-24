use coordinate_systems::Field;
use filtering::hysteresis::{
    greater_than_with_relative_hysteresis, less_than_with_relative_hysteresis,
};
use geometry::{Distance, line_segment::LineSegment};
use linear_algebra::point;
use ros_z::Message;
use serde::{Deserialize, Serialize};
use types::{
    behavior_tree::Status, field_dimensions::Half, motion_type::MotionType, obstacles::ObstacleKind,
};

use crate::node::Blackboard;

use super::geometry::{goal_mouth, goal_openings};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Message)]
pub enum QuickKickReason {
    NearOwnGoal,
    NearOpponentGoal,
    NearbyRobot,
    NoUsablePlan,
}

/// Tactical intent only. Execution chooses the target, approach, and physical motion.
/// A future pass variant can identify a receiver and reception region.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Message)]
pub enum BallAction {
    /// Release goalward immediately; execution still checks reach and motion switching.
    QuickKick {
        reason: QuickKickReason,
    },
    Shoot {
        opening: LineSegment<Field>,
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
/// Clear/dribble execution is unavailable; expose the deliberate quick fallback.
pub fn select_ball_action(blackboard: &mut Blackboard) -> Status {
    blackboard.ball_action = choose_ball_action(blackboard).map(|action| match action {
        BallAction::Dribble | BallAction::Clear => BallAction::QuickKick {
            reason: QuickKickReason::NoUsablePlan,
        },
        action => action,
    });
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
    let previous = blackboard.ball_action.or(blackboard.last_ball_action);
    let ground_to_field = world.robot.ground_to_field?;
    let goal_x = field.length / 2.0;
    if blackboard.last_motion_type == Some(MotionType::Kick) && previous.is_some() {
        return previous;
    }
    if let Some(reason) = urgent_kick_reason(blackboard) {
        return Some(BallAction::QuickKick { reason });
    }
    if ball.x().abs() >= goal_x || ball.y().abs() >= field.width / 2.0 {
        return None;
    }

    let was_clearing = matches!(previous, Some(BallAction::Clear));
    let own_goal = LineSegment(
        point![-goal_x, -field.goal_inner_width / 2.0],
        point![-goal_x, field.goal_inner_width / 2.0],
    );
    let ball_in_ground = ground_to_field.inverse() * ball;
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
    if under_pressure
        && less_than_with_relative_hysteresis(
            was_clearing,
            own_goal.distance_to(ball),
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
            return Some(BallAction::Shoot { opening });
        }
    }

    Some(BallAction::Dribble)
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
                && state.is_some_and(|state| (state.pose.position() - robot).norm() <= 1.5)
        });
    (nearby_obstacle || nearby_teammate).then_some(QuickKickReason::NearbyRobot)
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
            obstacle_clearance: 0.05,
            minimum_shot_angle: 0.02,
            shot_angle_hysteresis: 0.05,
            pressure_distance: 0.8,
            clear_goal_distance: 1.5,
            clear_distance_hysteresis: 0.2,
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
        assert_eq!(
            blackboard.ball_action,
            Some(BallAction::QuickKick {
                reason: QuickKickReason::NoUsablePlan
            })
        );
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
}
