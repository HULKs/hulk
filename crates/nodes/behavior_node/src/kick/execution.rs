use coordinate_systems::{Field, Ground};
use geometry::{line::Line2, line_segment::LineSegment};
use linear_algebra::{Isometry2, Orientation2, Point2, Pose2, Vector2, point, vector};
use types::{
    behavior_tree::Status,
    motion_command::{BodyMotion, OrientationMode},
    motion_type::MotionType,
    path::traits::PathProgress,
};

use crate::{
    actions::stand, conditions::is_close_to_ball, node::Blackboard,
    switch_motion_type::is_allowed_to_start_kick, walk::walk_to,
};

use super::{
    geometry::{BALL_FIELD_BOUNDARY_TOLERANCE, goal_openings, pass_openings, travel_openings},
    kick_ball_in_ground,
    passing::{is_executing_announced_pass, pass_is_announced, pass_receiver_position},
    strategy::{BallAction, own_non_penalty_restart, select_restart_ball_action},
};

pub fn finish_close_goal(blackboard: &mut Blackboard) -> Status {
    let Some(transform) = blackboard.world_state.robot.ground_to_field else {
        return Status::Failure;
    };
    let Some(ball) = super::kick_ball_position_in_ground(blackboard, &transform) else {
        return Status::Failure;
    };
    let position = transform * ball;
    let field = blackboard.field_dimensions;
    let goal_x = field.length / 2.0;
    let half_width = field.goal_inner_width / 2.0 - field.ball_radius;
    if blackboard.player_number == blackboard.parameters.goalkeeper.player_number
        || blackboard.parameters.kicking.strategy.validate().is_err()
        || !position.inner.iter().all(|value| value.is_finite())
        || !field.length.is_finite()
        || field.length <= 0.0
        || !field.goal_box_area_length.is_finite()
        || field.goal_box_area_length <= 0.0
        || !half_width.is_finite()
        || half_width <= 0.0
        || !(goal_x - field.goal_box_area_length..goal_x).contains(&position.x())
        || position.y().abs() >= half_width
        || ball.x() <= 0.0
        || ball.coords().norm() > blackboard.parameters.kicking.kick_activation_distance
        || (transform * vector![1.0, 0.0]).x() <= 0.0
        || (blackboard.last_motion_type != Some(MotionType::Kick)
            && !is_allowed_to_start_kick(blackboard))
    {
        return Status::Failure;
    }
    let target = goal_openings(
        position,
        transform,
        &blackboard.world_state.obstacles,
        &field,
        blackboard.parameters.kicking.strategy.obstacle_clearance,
    )
    .into_iter()
    .max_by(|a, b| {
        LineSegment(position, a.0)
            .angle(LineSegment(position, a.1))
            .total_cmp(&LineSegment(position, b.0).angle(LineSegment(position, b.1)))
    })
    .map_or(point![goal_x, 0.0], |opening| opening.center());
    // At the goal mouth, release even when no clear corridor remains.
    if super::kick(blackboard) == Status::Failure {
        return Status::Failure;
    }
    blackboard.ball_action = None;
    super::apply_kick_target(blackboard, target);
    if let Some(BodyMotion::Kick {
        quick,
        strong,
        target_speed,
        ..
    }) = &mut blackboard.body_motion
    {
        *quick = true;
        *strong = false;
        *target_speed = blackboard.parameters.kicking.target_speed;
    }
    super::kick_strength_subtree().tick_with_trace(blackboard).0
}

/// Revalidate restart passes, dribbles, and clearances before every kick.
/// Ordinary goal shots keep the base kick subtree.
pub fn execute_ball_action(blackboard: &mut Blackboard) -> Status {
    if matches!(
        blackboard.ball_action,
        Some(BallAction::Dribble | BallAction::Clear)
    ) {
        return execute_travel_action(blackboard);
    }
    if !matches!(blackboard.ball_action, Some(BallAction::Pass { .. })) {
        return Status::Failure;
    }
    let target = resolve_target(blackboard).or_else(|| {
        // Reselect only while our restart is still active.
        if own_non_penalty_restart(blackboard) {
            blackboard.last_announced_pass = None;
            select_restart_ball_action(blackboard);
            resolve_target(blackboard)
        } else {
            None
        }
    });
    let Some(target_in_field) = target else {
        blackboard.ball_action = None;
        blackboard.kick_target = None;
        blackboard.last_announced_pass = None;
        return stand(blackboard);
    };
    let Some(BallAction::Pass { receiver }) = blackboard.ball_action else {
        return Status::Failure;
    };
    let Some(transform) = blackboard.world_state.robot.ground_to_field else {
        return Status::Failure;
    };
    let Some((ball, ball_velocity)) = kick_ball_in_ground(blackboard, &transform) else {
        return Status::Failure;
    };
    let target = transform.inverse() * target_in_field;
    blackboard.kick_target = Some(target);
    let Some(direction) = (target - ball).try_normalize(f32::EPSILON) else {
        return stand(blackboard);
    };
    let orientation = Orientation2::from_vector(direction);
    let kicking = &blackboard.parameters.kicking;
    let strong = kicking.allow_strong_kicks
        && (super::kickoff::own_kickoff(blackboard)
            || (target - ball).norm()
                >= kicking
                    .strong_kick_min_target_distance
                    .max(blackboard.field_dimensions.penalty_area_width));
    let target_speed = kicking.target_speed;
    let was_kicking = blackboard.last_motion_type == Some(MotionType::Kick);
    let can_switch = is_allowed_to_start_kick(blackboard);
    if ((ball.x() >= 0.0 && is_close_to_ball(blackboard)) || (was_kicking && !can_switch))
        && (was_kicking || can_switch)
    {
        if !pass_is_announced(blackboard, receiver, target_in_field) {
            return stand(blackboard);
        }
        blackboard.body_motion = Some(BodyMotion::Kick {
            ball_position: ball,
            ball_velocity,
            kick_direction: orientation,
            target_speed,
            soft: false,
            quick: false,
            strong,
        });
        blackboard.last_kick_target = Some(target_in_field);
        if was_kicking {
            super::use_last_kick_settings(blackboard);
        }
        return Status::Success;
    }
    approach_ball(blackboard, ball)
}

fn execute_travel_action(blackboard: &mut Blackboard) -> Status {
    let mut target = resolve_target(blackboard);
    if target.is_none() && !own_non_penalty_restart(blackboard) {
        super::strategy::select_ball_action(blackboard);
        if !matches!(
            blackboard.ball_action,
            Some(BallAction::Dribble | BallAction::Clear)
        ) {
            blackboard.kick_target = None;
            return if blackboard.ball_action.is_none() {
                stand(blackboard)
            } else {
                Status::Failure
            };
        }
        target = resolve_target(blackboard);
    }
    let Some(target) = target else {
        blackboard.ball_action = None;
        blackboard.kick_target = None;
        return stand(blackboard);
    };
    let Some(transform) = blackboard.world_state.robot.ground_to_field else {
        return Status::Failure;
    };
    let Some(ball) = super::kick_ball_position_in_ground(blackboard, &transform) else {
        return Status::Failure;
    };
    blackboard.kick_target = Some(transform.inverse() * target);
    let was_kicking = blackboard.last_motion_type == Some(MotionType::Kick);
    let can_switch = is_allowed_to_start_kick(blackboard);
    let ready = !super::reception::is_receiving_ball(blackboard)
        || super::reception::is_ready_to_receive_kick(blackboard);
    if ((ball.x() >= 0.0 && is_close_to_ball(blackboard) && ready) || (was_kicking && !can_switch))
        && (was_kicking || can_switch)
    {
        if super::kick(blackboard) == Status::Failure {
            return Status::Failure;
        }
        super::apply_kick_target(blackboard, target);
        blackboard.last_kick_target = Some(target);
        if !was_kicking {
            let dribbling = blackboard.ball_action == Some(BallAction::Dribble);
            let field_ball = transform * ball;
            let field = &blackboard.field_dimensions;
            let urgent = (field_ball.x() < -field.length / 2.0 + field.penalty_area_length
                && field_ball.y().abs() < field.penalty_area_width / 2.0)
                || crate::conditions::opponent_near_position(
                    blackboard,
                    field_ball,
                    blackboard.parameters.kicking.strategy.pressure_distance,
                );
            let kicking = &blackboard.parameters.kicking;
            if let Some(BodyMotion::Kick {
                quick,
                strong,
                soft,
                target_speed,
                ..
            }) = &mut blackboard.body_motion
            {
                *quick = !dribbling && (kicking.quick || urgent);
                *strong = false;
                *soft = false;
                *target_speed = if dribbling {
                    kicking.dribble_target_speed.min(kicking.target_speed)
                } else {
                    kicking.target_speed
                };
            }
            if !dribbling {
                super::kick_strength_subtree().tick_with_trace(blackboard);
            }
        }
        return Status::Success;
    }
    approach_ball(blackboard, ball)
}

pub(crate) fn approach_restart_ball(blackboard: &mut Blackboard) -> Status {
    if !own_non_penalty_restart(blackboard) {
        return Status::Failure;
    }
    if is_close_to_ball(blackboard) {
        return stand(blackboard);
    }
    let Some(transform) = blackboard.world_state.robot.ground_to_field else {
        return Status::Failure;
    };
    let Some((ball, _)) = kick_ball_in_ground(blackboard, &transform) else {
        return Status::Failure;
    };
    approach_ball(blackboard, ball)
}

fn approach_ball(blackboard: &mut Blackboard, ball: Point2<Ground>) -> Status {
    if blackboard.last_motion_type != Some(MotionType::Walk)
        && !is_allowed_to_start_kick(blackboard)
    {
        return stand(blackboard);
    }
    // Approach the observed ball; the kick policy handles final alignment.
    let Some(toward_ball) = ball.coords().try_normalize(f32::EPSILON) else {
        return stand(blackboard);
    };
    let approach = ball - toward_ball * blackboard.parameters.kicking.approach_ball_standoff;
    let walking = blackboard.parameters.walking.walk_and_stand;
    let status = walk_to(
        blackboard,
        Pose2::from_parts(approach, Orientation2::from_vector(ball.coords())),
        blackboard.parameters.walking.speed.kicking,
        OrientationMode::LookAt {
            target: ball,
            tolerance: walking.orientation_tolerance,
        },
        walking.normal_distance_to_be_aligned,
        walking.hysteresis,
    );
    if let Some(BodyMotion::Walk { path, speed, .. }) = &mut blackboard.body_motion {
        *speed = blackboard.parameters.walking.speed.kicking;
        if path.forward(Point2::origin()).x() < 0.0 {
            blackboard.body_motion = Some(BodyMotion::WalkWithVelocity {
                velocity: Vector2::zeros(),
                angular_velocity: blackboard
                    .walking_velocity_limits
                    .scale_angular_axis(Orientation2::from_vector(ball.coords()).angle()),
            });
        }
    }
    status
}

pub(crate) fn resolve_target(blackboard: &Blackboard) -> Option<Point2<Field>> {
    let action = blackboard.ball_action?;
    let transform = blackboard.world_state.robot.ground_to_field?;
    let ball = transform
        * if matches!(action, BallAction::Dribble | BallAction::Clear) {
            super::kick_ball_position_in_ground(blackboard, &transform)?
        } else {
            kick_ball_in_ground(blackboard, &transform)?.0
        };
    let field = &blackboard.field_dimensions;
    let parameters = &blackboard.parameters.kicking.strategy;
    parameters.validate().ok()?;
    if !ball.inner.iter().all(|value| value.is_finite())
        || !field.length.is_finite()
        || !field.width.is_finite()
        || field.length <= 0.0
        || field.width <= 0.0
        || ball.x().abs() > field.length / 2.0 + BALL_FIELD_BOUNDARY_TOLERANCE
        || ball.y().abs() > field.width / 2.0 + BALL_FIELD_BOUNDARY_TOLERANCE
    {
        return None;
    }
    match action {
        BallAction::Pass { receiver } => {
            if !blackboard.has_team_message_budget() {
                return None;
            }
            if let Some(target) = super::kickoff::kickoff_pass_target(blackboard, receiver) {
                return Some(target);
            }
            let executing = is_executing_announced_pass(blackboard, receiver);
            if !own_non_penalty_restart(blackboard) && !executing {
                return None;
            }
            let receiver_position = pass_receiver_position(blackboard, ball, receiver)?;
            if !executing
                && !crate::substates::restart_pass_is_open(blackboard, receiver_position)
                && blackboard
                    .world_state
                    .filtered_game_controller_state
                    .as_ref()
                    .is_some_and(|game| {
                        matches!(
                            game.sub_state,
                            Some(
                                hsl_network_messages::SubState::CornerKick
                                    | hsl_network_messages::SubState::ThrowIn
                            )
                        )
                    })
            {
                return None;
            }
            let mut passing = parameters.passing.clone();
            if executing {
                passing.minimum_distance = 0.0;
            }
            pass_openings(
                ball,
                receiver_position,
                transform,
                &blackboard.world_state.obstacles,
                field,
                &passing,
                parameters.obstacle_clearance,
            )
            .into_iter()
            .filter_map(|opening| {
                let direction = (opening.0 - ball).normalize() + (opening.1 - ball).normalize();
                let target =
                    geometry::line::Line2::from(opening).intersection(&geometry::line::Line2 {
                        point: ball,
                        direction,
                    });
                target
                    .inner
                    .iter()
                    .all(|value| value.is_finite())
                    .then_some((
                        target,
                        LineSegment(ball, opening.0).angle(LineSegment(ball, opening.1)),
                    ))
            })
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(target, _)| target)
        }
        BallAction::Shoot { opening } => {
            let range = parameters.maximum_shot_distance;
            let dx = field.length / 2.0 - ball.x();
            if dx < 0.0 || dx >= range {
                return None;
            }
            let reachable_y = (range.powi(2) - dx.powi(2)).sqrt();
            goal_openings(
                ball,
                transform,
                &blackboard.world_state.obstacles,
                field,
                parameters.obstacle_clearance,
            )
            .into_iter()
            .filter_map(|visible| {
                let lower = opening.0.y().max(visible.0.y()).max(ball.y() - reachable_y);
                let upper = opening.1.y().min(visible.1.y()).min(ball.y() + reachable_y);
                if lower >= upper {
                    return None;
                }
                let refined =
                    LineSegment(point![opening.0.x(), lower], point![opening.1.x(), upper]);
                let angle = LineSegment(ball, refined.0).angle(LineSegment(ball, refined.1));
                (angle >= parameters.minimum_shot_angle).then_some((refined.center(), angle))
            })
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(target, _)| target)
        }
        BallAction::QuickKick { .. } => {
            Some(if blackboard.last_motion_type == Some(MotionType::Kick) {
                blackboard
                    .last_kick_target
                    .unwrap_or(point![field.length / 2.0, 0.0])
            } else {
                point![field.length / 2.0, 0.0]
            })
        }
        BallAction::Clear | BallAction::Dribble => {
            travel_target(blackboard, action, transform, ball)
        }
    }
}

pub(super) fn travel_target(
    blackboard: &Blackboard,
    action: BallAction,
    transform: Isometry2<Ground, Field>,
    ball: Point2<Field>,
) -> Option<Point2<Field>> {
    let parameters = &blackboard.parameters.kicking;
    let distance = if action == BallAction::Dribble {
        if !parameters.dribble_target_speed.is_finite() || parameters.dribble_target_speed <= 0.0 {
            return None;
        }
        parameters.strategy.maximum_dribble_distance
    } else {
        parameters
            .strategy
            .maximum_shot_distance
            .min(blackboard.field_dimensions.length / 2.0)
    };
    let goal = point![blackboard.field_dimensions.length / 2.0, 0.0];
    travel_openings(
        ball,
        transform,
        &blackboard.world_state.obstacles,
        &blackboard.field_dimensions,
        distance,
        parameters.strategy.obstacle_clearance,
    )
    .into_iter()
    .filter(|opening| opening.length() > f32::EPSILON)
    .map(|opening| goalward_target(ball, goal, opening))
    .filter(|target| target.inner.iter().all(|value| value.is_finite()))
    .filter(|target| action != BallAction::Clear || target.x() > ball.x())
    .min_by(|a, b| {
        (a - ball)
            .angle(&(goal - ball))
            .total_cmp(&(b - ball).angle(&(goal - ball)))
    })
}

fn goalward_target(
    ball: Point2<Field>,
    goal: Point2<Field>,
    opening: LineSegment<Field>,
) -> Point2<Field> {
    // Stay inside tangent boundaries, including when the preferred ray is blocked.
    let inset = (opening.1 - opening.0) * (0.001 / opening.length()).min(0.5);
    let opening = LineSegment(opening.0 + inset, opening.1 - inset);
    let preferred = Line2::from(opening).intersection(&Line2::from_points(ball, goal));
    if (0.0..=1.0).contains(&opening.projection_factor(preferred))
        && (preferred - ball).dot(&(goal - ball)) > 0.0
    {
        preferred
    } else if (opening.0 - ball).angle(&(goal - ball)) <= (opening.1 - ball).angle(&(goal - ball)) {
        opening.0
    } else {
        opening.1
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use geometry::Distance;
    use hsl_network_messages::SubState;
    use linear_algebra::{Pose2, vector};
    use types::{ball_position::BallPosition, obstacles::Obstacle, world_state::BallState};

    use super::*;
    use crate::{
        kick::strategy::{QuickKickReason, select_ball_action, urgent_kick_reason},
        tree::create_tree,
    };

    fn board(robot_x: f32) -> Blackboard {
        let mut board = crate::test_support::restart_blackboard(
            SubState::DirectFreeKick,
            point![robot_x + 0.6, 0.0],
        );
        board.field_dimensions.length = 14.0;
        board.field_dimensions.width = 9.0;
        board.parameters.control.is_simple = true;
        board
            .world_state
            .filtered_game_controller_state
            .as_mut()
            .unwrap()
            .sub_state = None;
        board.world_state.robot.ground_to_field =
            Some(Pose2::new(point![robot_x, 0.0], 0.0).as_transform());
        board
    }

    #[test]
    fn live_goal_mouth_finishing_uses_the_open_side_and_shoots_when_fully_blocked() {
        for blocked in [false, true] {
            let mut board = board(6.2);
            let transform = board.world_state.robot.ground_to_field.unwrap();
            board.parameters.kicking.quick = false;
            let obstacle = if blocked {
                Obstacle::robot(transform.inverse() * point![6.95, 0.0], 0.2, 0.2)
            } else {
                Obstacle::robot(transform.inverse() * point![6.95, 0.15], 0.03, 0.03)
            };
            board.world_state.obstacles = vec![obstacle];
            let (status, _) = create_tree().tick_with_trace(&mut board);
            assert_eq!(status, Status::Success);
            assert!(board.ball_action.is_none());
            assert!(matches!(
                board.body_motion,
                Some(BodyMotion::Kick {
                    quick: true,
                    strong: false,
                    target_speed: 3.4,
                    ..
                })
            ));
            let target = transform * board.kick_target.unwrap();
            assert!((target.x() - 7.0).abs() < 1e-5);
            if blocked {
                assert!(target.y().abs() < 1e-5);
            } else {
                assert!(target.y() < -0.1);
                assert!(
                    LineSegment(point![6.8, 0.0], target)
                        .distance_to(transform * obstacle.position)
                        > obstacle.radius_at_foot_height
                            + board.field_dimensions.ball_radius
                            + board.parameters.kicking.strategy.obstacle_clearance
                );
            }
        }
    }

    #[test]
    fn goal_mouth_finishing_keeps_visual_reach_role_area_and_motion_guards() {
        let base = board(6.2);
        for case in 0..6 {
            let mut board = base.clone();
            match case {
                0 => board.player_number = board.parameters.goalkeeper.player_number,
                1 => board.ball.as_mut().unwrap().position = point![5.0, 0.0],
                2 => board.ball.as_mut().unwrap().position = point![6.8, 1.0],
                3 => {
                    board.visual_kick_ball_position = Some(BallPosition {
                        position: point![2.0, 0.0],
                        velocity: Vector2::zeros(),
                        last_seen: board.world_state.now,
                    })
                }
                4 => {
                    board.world_state.robot.ground_to_field =
                        Some(Pose2::new(point![7.4, 0.0], std::f32::consts::PI).as_transform())
                }
                _ => {
                    board.last_motion_type = Some(MotionType::StandUp);
                    board.last_motion_switch_time = board.world_state.now;
                    board.parameters.control.allow_switch.stand_up = Duration::from_secs(1);
                }
            }
            assert_eq!(
                finish_close_goal(&mut board),
                Status::Failure,
                "case {case}"
            );
            assert!(board.body_motion.is_none());
        }
    }

    #[test]
    fn live_tree_executes_controlled_dribbles_and_pressured_clearances_despite_urgency() {
        for clearing in [false, true] {
            let mut board = board(if clearing { -6.0 } else { -4.5 });
            if clearing {
                board.parameters.kicking.quick = false;
                board.world_state.obstacles = vec![Obstacle::robot(point![1.2, 0.1], 0.2, 0.2)];
            }
            assert!(urgent_kick_reason(&board).is_some());
            let (status, _) = create_tree().tick_with_trace(&mut board);
            assert_eq!(status, Status::Success);
            assert_eq!(
                board.ball_action,
                Some(if clearing {
                    BallAction::Clear
                } else {
                    BallAction::Dribble
                })
            );
            let Some(BodyMotion::Kick {
                quick,
                strong,
                soft: false,
                target_speed,
                ..
            }) = board.body_motion
            else {
                panic!("expected tactical kick");
            };
            assert_eq!((quick, strong), (clearing, clearing));
            assert_eq!(target_speed, if clearing { 3.4 } else { 0.5 });
            let transform = board.world_state.robot.ground_to_field.unwrap();
            let target = transform * board.kick_target.unwrap();
            let ball = board.ball.as_ref().unwrap().position;
            assert!(target.x() > ball.x());
            for obstacle in &board.world_state.obstacles {
                assert!(
                    LineSegment(ball, target).distance_to(transform * obstacle.position)
                        > obstacle.radius_at_foot_height
                            + board.field_dimensions.ball_radius
                            + board.parameters.kicking.strategy.obstacle_clearance
                );
            }
        }
    }

    #[test]
    fn travel_revalidates_blockers_keeps_end_line_dribbles_infield_and_honors_disabling() {
        let mut board = board(0.0);
        select_ball_action(&mut board);
        assert_eq!(board.ball_action, Some(BallAction::Dribble));
        execute_ball_action(&mut board);
        let clear_target = board.kick_target.unwrap();
        board.world_state.obstacles = vec![Obstacle::robot(point![1.8, 0.0], 0.2, 0.2)];
        execute_ball_action(&mut board);
        let target = board.kick_target.unwrap();
        assert_ne!(target, clear_target);
        assert!(
            LineSegment(point![0.6, 0.0], target).distance_to(point![1.8, 0.0])
                > 0.2
                    + board.field_dimensions.ball_radius
                    + board.parameters.kicking.strategy.obstacle_clearance
        );

        board.world_state.obstacles = vec![Obstacle::robot(point![0.6, 0.0], 0.5, 0.5)];
        assert_eq!(execute_ball_action(&mut board), Status::Failure);
        assert!(matches!(
            board.ball_action,
            Some(BallAction::QuickKick { .. })
        ));
        assert!(board.kick_target.is_none());

        board.world_state.obstacles.clear();
        board.parameters.kicking.strategy.maximum_dribble_distance = 0.0;
        select_ball_action(&mut board);
        assert_eq!(
            board.ball_action,
            Some(BallAction::QuickKick {
                reason: QuickKickReason::NoUsablePlan
            })
        );
        board.parameters.kicking.strategy.maximum_dribble_distance = f32::NAN;
        board.ball_action = Some(BallAction::Dribble);
        execute_ball_action(&mut board);
        assert_eq!(board.body_motion, Some(BodyMotion::Stand));
        assert!(board.ball_action.is_none());

        board.parameters.kicking.strategy.maximum_dribble_distance = 3.0;
        let ball = point![6.95, 3.9];
        let transform = Pose2::new(ball - vector![0.6, 0.0], 0.0).as_transform();
        board.world_state.robot.ground_to_field = Some(transform);
        board.ball.as_mut().unwrap().position = ball;
        board.ball_action = Some(BallAction::Dribble);
        execute_ball_action(&mut board);
        let target = transform * board.kick_target.unwrap();
        let margin = board.field_dimensions.ball_radius
            + board.parameters.kicking.strategy.obstacle_clearance;
        assert!(target.x() < ball.x());
        assert!((target - ball).norm() >= 3.0 - 1e-5);
        assert!(target.x().abs() <= board.field_dimensions.length / 2.0 - margin);
        assert!(target.y().abs() <= board.field_dimensions.width / 2.0 - margin);
    }

    #[test]
    fn travel_kicks_preserve_visual_pairs_reach_and_recovery_locks() {
        for action in [BallAction::Dribble, BallAction::Clear] {
            let mut board = board(0.0);
            board.ball_action = Some(action);
            board.visual_kick_ball_position = Some(BallPosition {
                position: point![0.6, 0.0],
                velocity: vector![0.2, 0.1],
                last_seen: board.world_state.now,
            });
            board.world_state.ball = Some(BallState {
                ball_in_ground: point![3.0, 1.0],
                ball_in_ground_velocity: vector![2.0, 1.0],
                ..BallState::new_at_center(Default::default())
            });
            execute_ball_action(&mut board);
            assert!(
                matches!(board.body_motion, Some(BodyMotion::Kick { ball_position, ball_velocity, .. })
                if ball_position == point![0.6, 0.0] && ball_velocity == vector![0.2, 0.1])
            );

            board.visual_kick_ball_position.as_mut().unwrap().position = point![2.0, 0.0];
            board.body_motion = None;
            execute_ball_action(&mut board);
            assert!(!matches!(board.body_motion, Some(BodyMotion::Kick { .. })));
            board.visual_kick_ball_position.as_mut().unwrap().position = point![0.6, 0.0];
            for motion in [
                MotionType::StandUp,
                MotionType::Prepare,
                MotionType::Damping,
            ] {
                board.last_motion_type = Some(motion);
                board.last_motion_switch_time = board.world_state.now;
                board.parameters.control.allow_switch.stand_up = Duration::from_secs(1);
                board.parameters.control.allow_switch.prepare = Duration::from_secs(1);
                board.parameters.control.allow_switch.damping = Duration::from_secs(1);
                execute_ball_action(&mut board);
                assert_eq!(board.body_motion, Some(BodyMotion::Stand));
            }
        }
    }
}
