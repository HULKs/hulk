use coordinate_systems::{Field, Ground};
use geometry::{line::Line2, line_segment::LineSegment};
use hsl_network_messages::{SubState, Team};
use linear_algebra::{Isometry2, Orientation2, Point2, Pose2, Vector2, point, vector};
use types::{
    behavior_tree::Status,
    motion_command::{BodyMotion, MotionCommand, OrientationMode},
    motion_type::MotionType,
    path::traits::PathProgress,
};

use crate::{
    actions::stand,
    conditions::is_close_to_ball,
    node::Blackboard,
    switch_motion_type::{is_allowed_to_start_kick, is_allowed_to_switch},
    walk::walk_to,
};

use super::{
    geometry::{BALL_FIELD_BOUNDARY_TOLERANCE, goal_openings, pass_openings, travel_openings},
    kick_ball_in_ground, kick_ball_position_in_ground, pass_receiver_position,
    passing::pass_is_announced,
    strategy::BallAction,
};

pub fn finish_close_goal(blackboard: &mut Blackboard) -> Status {
    let Some(transform) = blackboard.world_state.robot.ground_to_field else {
        return Status::Failure;
    };
    let Some(ball) = kick_ball_position_in_ground(blackboard, &transform) else {
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
    // At the goal mouth, contest a blocked shot instead of waiting for a perfect corridor.
    if super::kick(blackboard) == Status::Failure {
        return Status::Failure;
    }
    super::apply_kick_target(blackboard, target);
    if let Some(BodyMotion::Kick {
        quick,
        strong,
        target_speed,
        ..
    }) = &mut blackboard.body_motion
    {
        *quick = true;
        *strong = blackboard.parameters.kicking.allow_strong_kicks;
        *target_speed = blackboard.parameters.kicking.target_speed;
    }
    Status::Success
}

pub fn challenge_opponent_ball(blackboard: &mut Blackboard) -> Status {
    let Some(transform) = blackboard.world_state.robot.ground_to_field else {
        return Status::Failure;
    };
    let Some(ball) = kick_ball_position_in_ground(blackboard, &transform) else {
        return Status::Failure;
    };
    let forward = transform * vector![1.0, 0.0];
    if blackboard.player_number == blackboard.parameters.goalkeeper.player_number
        || blackboard.parameters.kicking.strategy.validate().is_err()
        || !ball.inner.iter().all(|value| value.is_finite())
        || ball.x() <= 0.0
        || ball.coords().norm() > blackboard.parameters.kicking.kick_activation_distance
        || !forward.x().is_finite()
        || forward.x() <= 0.0
        || super::passing::is_pass_receiver(blackboard)
        || !crate::conditions::opponent_near_ball(
            blackboard,
            blackboard.parameters.kicking.strategy.pressure_distance,
        )
        || crate::conditions::goalkeeper_clearance_direction(blackboard).is_some()
        || (blackboard.last_motion_type != Some(MotionType::Kick)
            && !is_allowed_to_start_kick(blackboard))
    {
        return Status::Failure;
    }
    let mut target = point![blackboard.field_dimensions.length / 2.0, 0.0];
    let mut speed = blackboard.parameters.kicking.target_speed;
    if super::strategy::select_ball_action(blackboard) == Status::Success {
        let ready_pass = match blackboard.ball_action {
            Some(BallAction::Pass { receiver }) => resolve_target(blackboard)
                .is_some_and(|target| pass_is_announced(blackboard, receiver, target)),
            _ => false,
        };
        if ready_pass
            || matches!(
                blackboard.ball_action,
                Some(BallAction::Shoot { .. } | BallAction::ShootAtGoalCenter | BallAction::Clear)
            )
        {
            return execute_ball_action(blackboard);
        }
        if blackboard.ball_action == Some(BallAction::Dribble)
            && let Some(diagonal) = resolve_target(blackboard)
        {
            target = diagonal;
            // A short dribble corridor does not guarantee room for a full-speed release.
            speed = blackboard
                .parameters
                .kicking
                .dribble_target_speed
                .min(speed);
        }
    }
    // Contest blocked balls goalward even when our body faces a touchline.
    if super::kick(blackboard) == Status::Failure {
        return Status::Failure;
    }
    super::apply_kick_target(blackboard, target);
    if let Some(BodyMotion::Kick {
        quick,
        strong,
        soft,
        target_speed,
        ..
    }) = &mut blackboard.body_motion
    {
        *quick = true;
        *strong = false;
        *soft = false;
        *target_speed = speed;
    }
    Status::Success
}

/// Revalidate the selected corridor, choosing another safe action if it has disappeared.
pub fn execute_ball_action(blackboard: &mut Blackboard) -> Status {
    if crate::conditions::goalkeeper_clearance_direction(blackboard).is_some() {
        if blackboard.last_motion_type != Some(MotionType::Walk)
            && !is_allowed_to_switch(blackboard)
        {
            blackboard.ball_action = None;
            blackboard.kick_target = None;
            return stand(blackboard);
        }
        return super::passing::yield_goalkeeper_clearance(blackboard);
    }
    let target = resolve_target(blackboard).or_else(|| {
        if super::kickoff::needs_kickoff_updates(blackboard) {
            return None;
        }
        blackboard.last_announced_pass = None;
        super::strategy::select_ball_action(blackboard);
        resolve_target(blackboard)
    });
    let Some(target) = target else {
        blackboard.ball_action = None;
        blackboard.kick_target = None;
        blackboard.last_announced_pass = None;
        return stand(blackboard);
    };
    let ground_to_field = blackboard.world_state.robot.ground_to_field.unwrap();
    let (ball, ball_velocity) = kick_ball_in_ground(blackboard, &ground_to_field).unwrap();
    let settling_pass = super::reception::is_settling_pass(blackboard);
    let pass_ready = match blackboard.ball_action {
        Some(BallAction::Pass { receiver }) => pass_is_announced(blackboard, receiver, target),
        _ => true,
    };
    let target = ground_to_field.inverse() * target;
    blackboard.kick_target = Some(target);
    let direction = (target - ball).normalize();
    let orientation = Orientation2::from_vector(direction);
    let kicking = &blackboard.parameters.kicking;
    let walk = blackboard.parameters.walking.walk_and_stand;
    let controlled = matches!(
        blackboard.ball_action,
        Some(BallAction::Dribble | BallAction::Pass { .. })
    );
    let force_strong = matches!(
        blackboard.ball_action,
        Some(BallAction::Shoot { .. } | BallAction::ShootAtGoalCenter | BallAction::Clear)
    ) || (matches!(blackboard.ball_action, Some(BallAction::Pass { .. }))
        && (super::kickoff::own_kickoff(blackboard)
            || (target - ball).norm()
                >= kicking
                    .strong_kick_min_target_distance
                    .max(blackboard.field_dimensions.penalty_area_width)));
    let strong = kicking.allow_strong_kicks && force_strong;
    let was_kicking = blackboard.last_motion_type == Some(MotionType::Kick);
    let target_speed = if blackboard.ball_action == Some(BallAction::Dribble) {
        kicking.dribble_target_speed.min(kicking.target_speed)
    } else {
        kicking.target_speed
    };
    let receiving = super::reception::is_receiving_ball(blackboard);
    let ready_to_receive = !receiving || super::reception::is_ready_to_receive_kick(blackboard);
    let field_ball = ground_to_field * ball;
    let field = &blackboard.field_dimensions;
    let urgent = (field_ball.x() < -field.length / 2.0 + field.penalty_area_length
        && field_ball.y().abs() < field.penalty_area_width / 2.0)
        || crate::conditions::opponent_near_ball(blackboard, kicking.strategy.pressure_distance);
    let own_goal_kick = blackboard
        .world_state
        .filtered_game_controller_state
        .as_ref()
        .is_some_and(|game| {
            game.kicking_team == Some(Team::Hulks) && game.sub_state == Some(SubState::GoalKick)
        });
    let quick = !own_goal_kick
        && ((receiving
            && ready_to_receive
            && blackboard.player_number != blackboard.parameters.goalkeeper.player_number)
            || (!controlled
                && (kicking.quick
                    || urgent
                    || (was_kicking
                        && matches!(
                            blackboard.last_motion_command,
                            Some(MotionCommand::Kick { quick: true, .. })
                        )))));
    let calm_goal_shot = blackboard.player_number != blackboard.parameters.goalkeeper.player_number
        && matches!(
            blackboard.ball_action,
            Some(BallAction::Shoot { .. } | BallAction::ShootAtGoalCenter)
        )
        && !crate::conditions::opponent_near_position(
            blackboard,
            ground_to_field.as_pose().position(),
            kicking.receiving_opponent_distance,
        );
    let can_switch = is_allowed_to_start_kick(blackboard);
    let close_to_ball = is_close_to_ball(blackboard);
    let needs_outgoing_turn = (settling_pass || calm_goal_shot)
        && !was_kicking
        && (direction.x() <= 0.0 || direction.dot(&ball.coords()) <= 0.0);
    if !needs_outgoing_turn
        && ((ball.x() >= 0.0 && (was_kicking || !receiving) && close_to_ball)
            || (receiving && ready_to_receive)
            || (was_kicking && !can_switch))
        && (was_kicking || can_switch)
    {
        if !pass_ready {
            return stand(blackboard);
        }
        blackboard.body_motion = Some(BodyMotion::Kick {
            ball_position: ball,
            ball_velocity,
            kick_direction: orientation,
            target_speed,
            soft: false,
            quick,
            strong,
        });
        return Status::Success;
    }
    if blackboard.last_motion_type != Some(MotionType::Walk) && !can_switch {
        // Cancel a kick while waiting for another motion to become available.
        return stand(blackboard);
    }

    // Only get within kick range. The kick policy handles final alignment.
    let projected = ground_to_field.inverse()
        * blackboard
            .projected_ball_position(ground_to_field * ball, ground_to_field * ball_velocity);
    let approach = if needs_outgoing_turn {
        let clearance = blackboard.parameters.kicking.approach_ball_standoff.max(
            blackboard.parameters.walking.path_planning.robot_radius
                + blackboard.field_dimensions.ball_radius
                + 0.05,
        );
        if ball.coords().norm() < clearance - 0.05 {
            // Make room before turning around a trapped ball inside the planner's obstacle.
            let away = -ball
                .coords()
                .try_normalize(f32::EPSILON)
                .unwrap_or(vector![1.0, 0.0]);
            let (velocity, _) = blackboard.walking_velocity_limits.clamp_command(
                away * ((clearance - ball.coords().norm()) / 0.35)
                    .min(blackboard.parameters.walking.speed.kicking),
                0.0,
            );
            blackboard.body_motion = Some(BodyMotion::WalkWithVelocity {
                velocity,
                angular_velocity: 0.0,
            });
            return Status::Success;
        }
        ball - direction * clearance
    } else {
        let Some(toward_ball) = projected.coords().try_normalize(f32::EPSILON) else {
            return stand(blackboard);
        };
        projected - toward_ball * blackboard.parameters.kicking.approach_ball_standoff
    };
    let status = walk_to(
        blackboard,
        Pose2::from_parts(
            approach,
            if needs_outgoing_turn {
                orientation
            } else {
                Orientation2::from_vector(ball.coords())
            },
        ),
        blackboard.parameters.walking.speed.kicking,
        if needs_outgoing_turn {
            OrientationMode::LookTowards {
                direction: orientation,
                tolerance: walk.orientation_tolerance,
            }
        } else {
            OrientationMode::LookAt {
                target: ball,
                tolerance: walk.orientation_tolerance,
            }
        },
        walk.normal_distance_to_be_aligned,
        walk.hysteresis,
    );
    if let Some(BodyMotion::Walk { path, speed, .. }) = &mut blackboard.body_motion {
        // The kick policy takes over at range; do not slow the approach several metres early.
        if !needs_outgoing_turn {
            *speed = blackboard.parameters.walking.speed.kicking;
        }
        let forward = path.forward(Point2::origin());
        if !needs_outgoing_turn && forward.x() < 0.0 {
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

pub(super) fn resolve_target(blackboard: &Blackboard) -> Option<Point2<Field>> {
    let action = blackboard.ball_action?;
    let transform = blackboard.world_state.robot.ground_to_field?;
    let ball = transform * kick_ball_position_in_ground(blackboard, &transform)?;
    let field = &blackboard.field_dimensions;
    let parameters = &blackboard.parameters.kicking.strategy;
    parameters.validate().ok()?;
    if !ball.x().is_finite()
        || !ball.y().is_finite()
        || !field.length.is_finite()
        || !field.width.is_finite()
        || field.length <= 0.0
        || field.width <= 0.0
        || ball.x().abs() > field.length / 2.0 + BALL_FIELD_BOUNDARY_TOLERANCE
        || ball.y().abs() > field.width / 2.0 + BALL_FIELD_BOUNDARY_TOLERANCE
    {
        return None;
    }
    resolve_action_target(blackboard, action, transform, ball)
}

fn resolve_action_target(
    blackboard: &Blackboard,
    action: BallAction,
    transform: Isometry2<Ground, Field>,
    ball: Point2<Field>,
) -> Option<Point2<Field>> {
    let field = &blackboard.field_dimensions;
    let parameters = &blackboard.parameters.kicking.strategy;
    let openings = match action {
        BallAction::ShootAtGoalCenter => {
            let target = point![field.length / 2.0, 0.0];
            return ((target - ball).norm() <= parameters.maximum_shot_distance
                && goal_openings(
                    ball,
                    transform,
                    &blackboard.world_state.obstacles,
                    field,
                    parameters.obstacle_clearance,
                )
                .iter()
                .any(|opening| opening.0.y() < 0.0 && opening.1.y() > 0.0))
            .then_some(target);
        }
        BallAction::Shoot { opening: selected } => {
            let dx = field.length / 2.0 - ball.x();
            if parameters.maximum_shot_distance <= dx {
                return None;
            }
            let dy = (parameters.maximum_shot_distance.powi(2) - dx.powi(2)).sqrt();
            goal_openings(
                ball,
                transform,
                &blackboard.world_state.obstacles,
                field,
                parameters.obstacle_clearance,
            )
            .into_iter()
            .filter_map(|opening| {
                let lower = opening.0.y().max(selected.0.y()).max(ball.y() - dy);
                let upper = opening.1.y().min(selected.1.y()).min(ball.y() + dy);
                (lower < upper).then_some(LineSegment(
                    point![opening.0.x(), lower],
                    point![opening.0.x(), upper],
                ))
            })
            .filter(|opening| {
                LineSegment(ball, opening.0).angle(LineSegment(ball, opening.1))
                    >= parameters.minimum_shot_angle
            })
            .collect()
        }
        BallAction::Pass { .. } if !blackboard.has_team_message_budget() => return None,
        BallAction::Pass { receiver } => {
            if let Some(target) = super::kickoff::kickoff_pass_target(blackboard, receiver) {
                return Some(target);
            }
            let mut passing = parameters.passing.clone();
            // Finish a launched pass as its remaining travel shrinks; still check obstacles.
            if super::passing::is_executing_announced_pass(blackboard, receiver) {
                passing.minimum_distance = 0.0;
            }
            pass_openings(
                ball,
                pass_receiver_position(blackboard, ball, receiver)?,
                transform,
                &blackboard.world_state.obstacles,
                field,
                &passing,
                parameters.obstacle_clearance,
            )
        }
        BallAction::Dribble | BallAction::Clear => {
            let distance = if action == BallAction::Dribble {
                parameters.maximum_dribble_distance
            } else {
                parameters.maximum_shot_distance.min(field.length / 2.0)
            };
            travel_openings(
                ball,
                transform,
                &blackboard.world_state.obstacles,
                field,
                distance,
                parameters.obstacle_clearance,
            )
        }
    };
    if matches!(action, BallAction::Dribble | BallAction::Clear) {
        // Pressure releases should advance toward goal instead of favoring wide sideways gaps.
        let goal = point![field.length / 2.0, 0.0];
        return openings
            .into_iter()
            .map(|opening| goalward_target(ball, goal, opening))
            .filter(|target| action != BallAction::Clear || target.x() > ball.x())
            .min_by(|a, b| {
                (a - ball)
                    .angle(&(goal - ball))
                    .total_cmp(&(b - ball).angle(&(goal - ball)))
            });
    }

    openings
        .into_iter()
        .filter_map(|opening| {
            // Bisect the visible angle to maximize angular edge clearance.
            let direction = (opening.0 - ball).normalize() + (opening.1 - ball).normalize();
            let target = Line2::from(opening).intersection(&Line2 {
                point: ball,
                direction,
            });
            (target.x().is_finite() && target.y().is_finite()).then_some((
                target,
                LineSegment(ball, opening.0).angle(LineSegment(ball, opening.1)),
            ))
        })
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(target, _)| target)
}

pub(crate) fn goalward_target(
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
    use hsl_network_messages::{PassIntent, PlayerNumber};

    use super::*;
    use crate::node::LastBall;
    use types::{
        field_dimensions::{FieldDimensions, Side},
        obstacles::Obstacle,
        path::traits::EndPoints,
        time_wrapper::TimeWrapper,
        world_state::{BallSource, BallState, PlayerState},
    };

    #[test]
    fn sideline_duels_use_an_open_diagonal_or_contest_goalward_instead_of_body_forward() {
        for side in [-1.0, 1.0] {
            for blocked_at_ball in [false, true] {
                let field = FieldDimensions {
                    length: 22.0,
                    width: 14.0,
                    goal_inner_width: 3.0,
                    ball_radius: 0.11,
                    ..FieldDimensions::SPL_2025
                };
                let ball = point![2.0, side * if blocked_at_ball { 5.8 } else { 0.8 }];
                let transform = Isometry2::from_parts(
                    vector![1.8, ball.y() - side * 0.55],
                    side * 70.0_f32.to_radians(),
                );
                let opponent = if blocked_at_ball {
                    ball + vector![0.25, 0.0]
                } else {
                    ball + vector![0.7, 0.0]
                };
                let mut board = Blackboard {
                    field_dimensions: field,
                    player_number: PlayerNumber::Two,
                    last_motion_type: Some(MotionType::Walk),
                    ball: Some(LastBall {
                        position: ball,
                        velocity: Vector2::zeros(),
                        age: Default::default(),
                        field_side: Side::Left,
                        source: BallSource::Own,
                    }),
                    ..Default::default()
                };
                board.world_state.robot.ground_to_field = Some(transform);
                board.world_state.obstacles =
                    vec![Obstacle::robot(transform.inverse() * opponent, 0.2, 0.2)];
                board.parameters.kicking.strategy.maximum_shot_distance = 12.0;
                board.parameters.kicking.strategy.maximum_dribble_distance = 3.0;
                board.parameters.kicking.strategy.obstacle_clearance = 0.05;
                board.parameters.kicking.strategy.pressure_distance = 0.8;
                board.parameters.kicking.kick_activation_distance = 1.3;
                board.parameters.kicking.target_speed = 3.4;
                board.parameters.kicking.dribble_target_speed = 0.5;

                assert_eq!(challenge_opponent_ball(&mut board), Status::Success);
                let Some(BodyMotion::Kick {
                    kick_direction,
                    quick: true,
                    strong: false,
                    soft: false,
                    target_speed,
                    ..
                }) = board.body_motion
                else {
                    panic!("expected an immediate duel kick")
                };
                let direction = transform * kick_direction.as_unit_vector();
                assert!(direction.x() > 0.0 && direction.y() * side < 0.0);
                let target = transform * board.kick_target.unwrap();
                if blocked_at_ball {
                    assert!((target - point![11.0, 0.0]).norm() < 1e-5);
                    assert_eq!(target_speed, 3.4);
                } else {
                    assert!(LineSegment(ball, target).distance_to(opponent) > 0.36);
                    assert_eq!(target_speed, 0.5);
                }
            }
        }
    }

    #[test]
    fn calm_pass_receiver_gets_behind_ball_before_starting_a_rearward_shot() {
        let mut board = Blackboard {
            player_number: PlayerNumber::Two,
            field_dimensions: FieldDimensions::SPL_2025,
            last_motion_type: Some(MotionType::Walk),
            ball_action: Some(BallAction::ShootAtGoalCenter),
            ..Default::default()
        };
        board.parameters.goalkeeper.player_number = PlayerNumber::One;
        board.parameters.ball.last_ball_timeout = Duration::from_secs(1);
        board.parameters.ball.interception.minimum_ball_velocity = 0.3;
        board.parameters.kicking.allow_strong_kicks = true;
        board.parameters.kicking.kick_activation_distance = 1.3;
        board.parameters.kicking.approach_ball_standoff = 0.3;
        board.parameters.kicking.receiving_opponent_distance = 2.0;
        board.parameters.kicking.strategy.maximum_shot_distance = 12.0;
        board.parameters.kicking.strategy.passing.maximum_distance = 8.0;
        board.parameters.kicking.strategy.passing.receiver_standoff = 0.5;
        board.parameters.kicking.strategy.passing.target_half_width = 0.3;
        board.parameters.walking.path_planning.robot_radius = 0.4;
        board.parameters.walking.path_planning.ball_obstacle_radius = 0.1;
        board.parameters.walking.speed.kicking = 1.8;
        board.parameters.walking.speed.velocity_fade_distance = 3.0;
        board.walking_velocity_limits = types::walking_velocity_limits::WalkingVelocityLimits {
            forward_velocity_limits: [-0.5, 1.8],
            lateral_velocity_limit: 1.0,
            angular_velocity_limit: 2.0,
        };
        let position = point![-0.55, 0.0];
        board.world_state.player_states.three = Some(TimeWrapper {
            time: board.world_state.now,
            inner: PlayerState {
                pose: point![-3.0, 0.0].into(),
                pass: Some(PassIntent {
                    receiver: PlayerNumber::Two,
                    target: position,
                }),
                ..crate::test_support::player_state()
            },
        });
        for (position, velocity, world_sample) in [
            (position, Vector2::zeros(), true),
            (position, vector![0.29, 0.0], false),
            (point![-0.2, 0.0], Vector2::zeros(), true),
        ] {
            board
                .world_state
                .player_states
                .three
                .as_mut()
                .unwrap()
                .inner
                .pass = Some(PassIntent {
                receiver: PlayerNumber::Two,
                target: position,
            });
            let transform = Isometry2::from_parts(vector![0.0, 0.0], std::f32::consts::PI);
            board.world_state.robot.ground_to_field = Some(transform);
            board.ball = Some(LastBall {
                position,
                velocity,
                age: board.world_state.now,
                field_side: Side::Left,
                source: BallSource::Own,
            });
            board.world_state.ball = world_sample.then_some(BallState {
                ball_in_ground: transform.inverse() * position,
                ball_in_ground_velocity: transform.inverse() * velocity,
                ball_in_field: position,
                last_seen_ball: board.world_state.now.to_wallclock(),
                field_side: Side::Left,
                source: BallSource::Own,
            });
            execute_ball_action(&mut board);
            match board.body_motion {
                Some(BodyMotion::Walk { ref path, .. }) => {
                    assert!((transform * path.end_point()).x() < position.x() - 0.3);
                    for segment in &path.segments {
                        if let types::path::PathSegment::LineSegment(line) = segment {
                            assert!(line.distance_to(transform.inverse() * position) >= 0.4);
                        }
                    }
                }
                Some(BodyMotion::WalkWithVelocity {
                    velocity,
                    angular_velocity,
                }) => {
                    assert!(velocity.dot(&(transform.inverse() * position).coords()) < 0.0);
                    assert_eq!(angular_velocity, 0.0);
                }
                _ => panic!("a calm receiver must make room before the rearward shot"),
            }
            board
                .world_state
                .player_states
                .three
                .as_mut()
                .unwrap()
                .inner
                .pass = None;
            execute_ball_action(&mut board);
            assert!(matches!(
                board.body_motion,
                Some(BodyMotion::Walk { .. } | BodyMotion::WalkWithVelocity { .. })
            ));
            board.world_state.robot.ground_to_field =
                Some(Isometry2::from_parts(vector![-1.05, 0.0], 0.0));
            if let Some(ball) = &mut board.world_state.ball {
                let transform = board.world_state.robot.ground_to_field.unwrap();
                ball.ball_in_ground = transform.inverse() * position;
                ball.ball_in_ground_velocity = transform.inverse() * velocity;
            }
            execute_ball_action(&mut board);
            assert!(matches!(
                board.body_motion,
                Some(BodyMotion::Kick {
                    strong: true,
                    soft: false,
                    ..
                })
            ));
        }
    }

    #[test]
    fn forecast_pursuit_and_overshoot_keep_facing_observed_ball() {
        let mut board = Blackboard {
            player_number: PlayerNumber::Two,
            field_dimensions: FieldDimensions::SPL_2025,
            last_motion_type: Some(MotionType::Walk),
            ball_action: Some(BallAction::Dribble),
            ball: Some(LastBall {
                position: point![2.0, 0.2],
                velocity: vector![0.0, 2.0],
                age: Default::default(),
                field_side: Side::Left,
                source: BallSource::Own,
            }),
            ..Default::default()
        };
        board.world_state.robot.ground_to_field = Some(Default::default());
        board.parameters.goalkeeper.player_number = PlayerNumber::One;
        board.parameters.ball.last_ball_timeout = Duration::from_secs(1);
        board.parameters.ball.pursuit_lookahead = Duration::from_secs(1);
        board.parameters.kicking.kick_activation_distance = 0.5;
        board.parameters.kicking.approach_ball_standoff = 0.3;
        board.parameters.kicking.strategy.maximum_shot_distance = 12.0;
        board.parameters.kicking.strategy.maximum_dribble_distance = 1.0;
        board.walking_velocity_limits.angular_velocity_limit = 2.0;
        assert_eq!(execute_ball_action(&mut board), Status::Success);
        let Some(BodyMotion::Walk {
            orientation_mode: OrientationMode::LookAt { target, .. },
            target_orientation,
            ref path,
            ..
        }) = board.body_motion
        else {
            panic!("expected forecast pursuit facing the observed ball")
        };
        assert_eq!(target, point![2.0, 0.2]);
        assert!(path.end_point().y() > target.y());
        assert!((target_orientation.angle() - target.y().atan2(target.x())).abs() < 1e-6);

        board.ball.as_mut().unwrap().position = point![1.0, 0.7];
        board.ball.as_mut().unwrap().velocity = vector![-3.0, -3.0];
        board.ball_action = Some(BallAction::Dribble);
        assert_eq!(execute_ball_action(&mut board), Status::Success);
        assert!(matches!(board.body_motion,
            Some(BodyMotion::WalkWithVelocity { velocity, angular_velocity })
            if velocity == Vector2::zeros()
                && (angular_velocity - 2.0 * 0.7_f32.atan2(1.0)).abs() < 1e-6));
    }

    #[test]
    fn walking_hands_off_immediately_and_urgent_shots_and_clears_are_quick() {
        let mut board = Blackboard {
            field_dimensions: FieldDimensions::SPL_2025,
            ball_action: Some(BallAction::ShootAtGoalCenter),
            ball: Some(LastBall {
                position: point![0.5, 0.0],
                velocity: Vector2::zeros(),
                age: Default::default(),
                field_side: Side::Left,
                source: BallSource::Own,
            }),
            last_motion_type: Some(MotionType::Walk),
            ..Default::default()
        };
        board.world_state.robot.ground_to_field = Some(Default::default());
        board.world_state.now = ros_z::time::Time::from_nanos(1_000_000_000);
        board.last_motion_switch_time =
            board.world_state.now - std::time::Duration::from_millis(100);
        board.parameters.control.allow_switch.walk = std::time::Duration::from_millis(500);
        board.parameters.control.allow_switch.stand_up = std::time::Duration::from_millis(500);
        board.parameters.kicking.strategy.maximum_shot_distance = 12.0;
        board.parameters.kicking.strategy.pressure_distance = 0.8;
        board.parameters.kicking.kick_activation_distance = 1.3;
        board.parameters.kicking.allow_strong_kicks = true;
        board.parameters.kicking.strong_kick_min_target_distance = 3.0;
        assert_eq!(execute_ball_action(&mut board), Status::Success);
        assert!(matches!(
            board.body_motion,
            Some(BodyMotion::Kick {
                strong: true,
                quick: false,
                soft: false,
                ..
            })
        ));

        board.world_state.obstacles = vec![Obstacle::robot(point![0.4, 0.6], 0.1, 0.1)];
        execute_ball_action(&mut board);
        assert!(matches!(
            board.body_motion,
            Some(BodyMotion::Kick {
                strong: true,
                quick: true,
                ..
            })
        ));

        board.world_state.obstacles.clear();
        board.world_state.robot.ground_to_field =
            Some(Isometry2::from_parts(vector![-4.2, 0.0], 0.0));
        board.ball.as_mut().unwrap().position = point![-3.7, 0.0];
        board.ball_action = Some(BallAction::Clear);
        execute_ball_action(&mut board);
        assert!(matches!(
            board.body_motion,
            Some(BodyMotion::Kick {
                strong: true,
                quick: true,
                soft: false,
                ..
            })
        ));

        board.ball.as_mut().unwrap().velocity = vector![0.4, 0.0];
        execute_ball_action(&mut board);
        assert!(matches!(
            board.body_motion,
            Some(BodyMotion::Kick {
                strong: true,
                quick: true,
                soft: false,
                ..
            })
        ));

        board.world_state.filtered_game_controller_state = Some(
            types::filtered_game_controller_state::FilteredGameControllerState {
                kicking_team: Some(Team::Hulks),
                sub_state: Some(SubState::GoalKick),
                ..crate::test_support::filtered_game_controller_state()
            },
        );
        board.ball_action = Some(BallAction::ShootAtGoalCenter);
        execute_ball_action(&mut board);
        assert!(matches!(
            board.body_motion,
            Some(BodyMotion::Kick {
                quick: false,
                strong: true,
                ..
            })
        ));
        board.world_state.filtered_game_controller_state = None;

        board.last_motion_type = Some(MotionType::StandUp);
        execute_ball_action(&mut board);
        assert!(matches!(board.body_motion, Some(BodyMotion::Stand)));

        board.last_motion_type = Some(MotionType::Kick);
        board.last_motion_switch_time = board.world_state.now;
        board.walking_velocity_limits.angular_velocity_limit = 2.0;
        board.parameters.control.allow_switch.kick = std::time::Duration::from_millis(500);
        board.world_state.robot.ground_to_field =
            Some(Isometry2::from_parts(vector![-3.8, 0.0], 0.0));
        board.ball.as_mut().unwrap().position = point![-4.3, 0.0];
        board.ball.as_mut().unwrap().velocity = Vector2::zeros();
        execute_ball_action(&mut board);
        assert!(matches!(board.body_motion, Some(BodyMotion::Kick { .. })));
        board.world_state.now = board.world_state.now + std::time::Duration::from_millis(501);
        execute_ball_action(&mut board);
        assert!(matches!(
            board.body_motion,
            Some(BodyMotion::WalkWithVelocity { velocity, angular_velocity })
                if velocity == Vector2::zeros() && angular_velocity.abs() > 0.0
        ));
    }

    #[test]
    fn long_large_field_passes_are_strong_while_short_passes_stay_controlled() {
        let mut board = Blackboard {
            player_number: PlayerNumber::Two,
            field_dimensions: FieldDimensions {
                length: 22.0,
                width: 14.0,
                penalty_area_width: 7.0,
                ..FieldDimensions::SPL_2025
            },
            last_motion_type: Some(MotionType::Walk),
            ..Default::default()
        };
        board.parameters.goalkeeper.player_number = PlayerNumber::One;
        board.parameters.kicking.allow_strong_kicks = true;
        board.parameters.kicking.strong_kick_min_target_distance = 3.0;
        board.parameters.kicking.kick_activation_distance = 1.3;
        let passing = &mut board.parameters.kicking.strategy.passing;
        passing.maximum_distance = 12.0;
        passing.minimum_distance = 4.0;
        passing.receiver_standoff = 0.5;
        passing.target_half_width = 0.3;
        passing.maximum_receiver_age = Duration::from_secs(3);
        board.world_state.robot.ground_to_field = Some(Default::default());
        board.world_state.filtered_game_controller_state = Some(
            types::filtered_game_controller_state::FilteredGameControllerState {
                remaining_number_of_messages: 1000,
                ..crate::test_support::filtered_game_controller_state()
            },
        );
        board.ball = Some(LastBall {
            position: point![0.5, 0.0],
            velocity: Vector2::zeros(),
            age: board.world_state.now,
            field_side: Side::Left,
            source: BallSource::Own,
        });
        for (travel, expected_strong) in [(5.0, false), (8.0, true)] {
            board.world_state.player_states.three = Some(TimeWrapper {
                time: board.world_state.now,
                inner: PlayerState {
                    pose: Pose2::new(point![travel + 1.0, 0.0], 0.0),
                    ..crate::test_support::player_state()
                },
            });
            board.ball_action = Some(BallAction::Pass {
                receiver: PlayerNumber::Three,
            });
            board.last_announced_pass = Some(TimeWrapper {
                time: board.world_state.now,
                inner: PassIntent {
                    receiver: PlayerNumber::Three,
                    target: resolve_target(&board).unwrap(),
                },
            });
            board.last_sent_hsl_message_time = Some(board.world_state.now);
            assert_eq!(execute_ball_action(&mut board), Status::Success);
            assert!(matches!(board.body_motion,
                Some(BodyMotion::Kick { strong, quick: false, soft: false, .. })
                    if strong == expected_strong));
        }
    }

    #[test]
    fn goal_shots_are_strong_even_when_close_or_rolling_and_honor_quick() {
        let mut blackboard = Blackboard {
            field_dimensions: FieldDimensions::SPL_2025,
            ball_action: Some(BallAction::ShootAtGoalCenter),
            ball: Some(LastBall {
                position: point![0.5, 0.0],
                velocity: Vector2::zeros(),
                age: Default::default(),
                field_side: Side::Left,
                source: BallSource::Own,
            }),
            ..Default::default()
        };
        blackboard.world_state.robot.ground_to_field = Some(Default::default());
        blackboard.field_dimensions.length = 6.0;
        blackboard.parameters.kicking.target_speed = 3.4;
        blackboard.parameters.kicking.strategy.maximum_shot_distance = 12.0;
        blackboard.parameters.kicking.soft = true;
        blackboard.parameters.kicking.allow_strong_kicks = true;
        blackboard
            .parameters
            .kicking
            .strong_kick_min_target_distance = 3.0;
        blackboard.parameters.kicking.kick_activation_distance = 3.0;
        assert_eq!(execute_ball_action(&mut blackboard), Status::Success);
        assert!(matches!(
            blackboard.body_motion,
            Some(BodyMotion::Kick {
                strong: true,
                soft: false,
                target_speed: 3.4,
                ..
            })
        ));

        blackboard.parameters.ball.last_ball_timeout = std::time::Duration::from_secs(1);
        blackboard.ball.as_mut().unwrap().velocity = vector![-1.5, 0.0];
        blackboard.last_motion_type = Some(MotionType::Walk);
        execute_ball_action(&mut blackboard);
        assert!(matches!(
            blackboard.body_motion,
            Some(BodyMotion::Kick {
                strong: true,
                quick: false,
                soft: false,
                target_speed: 3.4,
                ..
            })
        ));
        blackboard.parameters.kicking.quick = true;
        execute_ball_action(&mut blackboard);
        assert!(matches!(
            blackboard.body_motion,
            Some(BodyMotion::Kick { quick: true, .. })
        ));
        blackboard.last_motion_command = Some(MotionCommand::from_partial_motions(
            blackboard.body_motion.clone().unwrap(),
            types::motion_command::HeadMotion::Center {
                image_region_target: types::motion_command::ImageRegion::Center,
            },
        ));
        blackboard.last_motion_type = Some(MotionType::Kick);
        if let Some(MotionCommand::Kick { strong, .. }) = &mut blackboard.last_motion_command {
            *strong = false;
        }
        blackboard.parameters.kicking.quick = false;
        blackboard.ball.as_mut().unwrap().velocity = Vector2::zeros();
        execute_ball_action(&mut blackboard);
        assert!(matches!(
            blackboard.body_motion,
            Some(BodyMotion::Kick {
                strong: true,
                quick: true,
                ..
            })
        ));
        blackboard.last_motion_type = Some(MotionType::Walk);
        blackboard.world_state.now =
            blackboard.world_state.now + std::time::Duration::from_millis(1);
        execute_ball_action(&mut blackboard);
        assert!(matches!(
            blackboard.body_motion,
            Some(BodyMotion::Kick {
                strong: true,
                quick: false,
                ..
            })
        ));

        blackboard.parameters.kicking.allow_strong_kicks = false;
        assert_eq!(execute_ball_action(&mut blackboard), Status::Success);
        assert!(matches!(
            blackboard.body_motion,
            Some(BodyMotion::Kick { strong: false, .. })
        ));

        blackboard.ball_action = Some(BallAction::Dribble);
        blackboard.parameters.kicking.allow_strong_kicks = true;
        blackboard
            .parameters
            .kicking
            .strategy
            .maximum_dribble_distance = 1.0;
        blackboard.parameters.kicking.dribble_target_speed = 1.2;
        blackboard.last_motion_type = Some(MotionType::Kick);
        assert!(matches!(
            blackboard.last_motion_command,
            Some(MotionCommand::Kick { quick: true, .. })
        ));
        execute_ball_action(&mut blackboard);
        assert!(matches!(
            blackboard.body_motion,
            Some(BodyMotion::Kick {
                strong: false,
                quick: false,
                soft: false,
                target_speed: 1.2,
                ..
            })
        ));
    }
}
