pub mod execution;
pub mod geometry;
pub mod kickoff;
pub mod passing;
pub mod reception;
pub mod strategy;

#[cfg(test)]
mod tests;

use ::geometry::{Distance, line_segment::LineSegment};
use coordinate_systems::{Field, Ground};
use linear_algebra::{Isometry2, Orientation2, Point2, Vector2, point, vector};
use types::{
    behavior_tree::Status,
    field_dimensions::Half,
    motion_command::{BodyMotion, HeadMotion, ImageRegion, MotionCommand},
    motion_type::MotionType,
};

use crate::{
    action,
    actions::stand,
    behavior_tree::Node,
    condition,
    conditions::is_close_to_ball,
    node::Blackboard,
    selection, sequence,
    substates::is_in_sub_state,
    subtree,
    switch_motion_type::{is_last_motion_type, switch_motion_type},
    walk::walk_to_ball,
};

use self::{
    geometry::{goal_mouth, goal_openings},
    strategy::{BallAction, QuickKickReason},
};

pub fn kick_subtree() -> Node<Blackboard> {
    switch_motion_type(
        MotionType::Kick,
        sequence!(
            action!(kick),
            selection!(
                action!(select_kick_target),
                sequence!(
                    condition!(is_close_to_ball),
                    action!(select_fallback_kick_target),
                    action!(use_quick_kick_fallback),
                )
            ),
            subtree!(kick_strength_subtree),
        ),
        subtree!(kick_alternatives_subtree),
    )
}

pub fn quick_kick_subtree() -> Node<Blackboard> {
    sequence!(
        condition!(is_close_to_ball),
        switch_motion_type(
            MotionType::Kick,
            sequence!(
                action!(kick),
                action!(select_fallback_kick_target),
                action!(use_quick_kick),
                subtree!(kick_strength_subtree),
            ),
            subtree!(kick_alternatives_subtree),
        )
    )
}

pub fn kick_alternatives_subtree() -> Node<Blackboard> {
    selection!(
        sequence!(
            condition!(is_last_motion_type, MotionType::Walk),
            action!(walk_to_ball)
        ),
        action!(stand)
    )
}

pub fn kick(blackboard: &mut Blackboard) -> Status {
    blackboard.kick_target = None;
    if let Some(ground_to_field) = &blackboard.world_state.robot.ground_to_field
        && let Some((ball_in_ground, ball_velocity)) =
            visual_kick_ball_in_ground(blackboard, ground_to_field)
    {
        blackboard.body_motion = Some(BodyMotion::Kick {
            ball_position: ball_in_ground,
            ball_velocity,
            target_speed: blackboard.parameters.kicking.target_speed,
            soft: blackboard.parameters.kicking.soft,
            quick: blackboard.parameters.kicking.quick,
            kick_direction: Default::default(),
            strong: false,
        });
        if blackboard.last_motion_type == Some(MotionType::Kick) {
            use_last_kick_settings(blackboard);
        }
        blackboard.head_motion = Some(HeadMotion::LookAt {
            target: ball_in_ground,
            height_above_ground: blackboard.field_dimensions.ball_radius,
            image_region_target: ImageRegion::Center,
        });

        Status::Success
    } else {
        Status::Failure
    }
}

pub fn select_kick_target(blackboard: &mut Blackboard) -> Status {
    if blackboard.last_motion_type == Some(MotionType::Kick)
        || is_in_sub_state(blackboard)
        || blackboard.ball_action.is_none()
    {
        return select_fallback_kick_target(blackboard);
    }
    let Some(BallAction::Shoot { opening }) = blackboard.ball_action else {
        return Status::Failure;
    };
    let Some(ground_to_field) = blackboard.world_state.robot.ground_to_field else {
        return Status::Failure;
    };
    let Some(BodyMotion::Kick { ball_position, .. }) = blackboard.body_motion else {
        return Status::Failure;
    };
    let ball = ground_to_field * ball_position;
    let parameters = &blackboard.parameters.kicking.strategy;
    let range = parameters.maximum_shot_distance;
    let dx = blackboard.field_dimensions.length / 2.0 - ball.x();
    if !range.is_finite() || dx < 0.0 || dx >= range {
        return Status::Failure;
    }
    let reachable_y = (range.powi(2) - dx.powi(2)).sqrt();
    let target = goal_openings(
        ball,
        ground_to_field,
        &blackboard.world_state.obstacles,
        &blackboard.field_dimensions,
        parameters.obstacle_clearance,
    )
    .into_iter()
    .filter_map(|visible| {
        let lower = opening.0.y().max(visible.0.y()).max(ball.y() - reachable_y);
        let upper = opening.1.y().min(visible.1.y()).min(ball.y() + reachable_y);
        if lower >= upper {
            return None;
        }
        let refined = LineSegment(point![opening.0.x(), lower], point![opening.1.x(), upper]);
        let angle = LineSegment(ball, refined.0).angle(LineSegment(ball, refined.1));
        Some((refined.center(), angle))
    })
    .filter(|(_, angle)| *angle >= parameters.minimum_shot_angle)
    .max_by(|(_, a), (_, b)| a.total_cmp(b));
    match target {
        Some((target, _)) => apply_kick_target(blackboard, target),
        None => Status::Failure,
    }
}

pub fn use_quick_kick_fallback(blackboard: &mut Blackboard) -> Status {
    let status = use_quick_kick(blackboard);
    if status == Status::Success && blackboard.last_motion_type != Some(MotionType::Kick) {
        blackboard.ball_action = Some(BallAction::QuickKick {
            reason: QuickKickReason::NoUsablePlan,
        });
    }
    status
}

pub fn select_fallback_kick_target(blackboard: &mut Blackboard) -> Status {
    let goal_position: Point2<Field> = point!(blackboard.field_dimensions.length / 2.0, 0.0);
    let target = if blackboard.last_motion_type == Some(MotionType::Kick) {
        blackboard.last_kick_target.unwrap_or(goal_position)
    } else {
        goal_position
    };

    apply_kick_target(blackboard, target)
}

pub fn use_quick_kick(blackboard: &mut Blackboard) -> Status {
    if blackboard.last_motion_type == Some(MotionType::Kick) {
        return use_last_kick_settings(blackboard);
    }
    if let Some(BodyMotion::Kick { quick, .. }) = blackboard.body_motion.as_mut() {
        *quick = true;
        return Status::Success;
    }
    Status::Failure
}

pub fn kickoff_kick(blackboard: &mut Blackboard) -> Status {
    if kick(blackboard) == Status::Failure {
        return Status::Failure;
    }
    if let Some(ground_to_field) = blackboard.world_state.robot.ground_to_field
        && let Some(BodyMotion::Kick {
            ball_position,
            target_speed,
            soft,
            quick,
            strong,
            ..
        }) = blackboard.body_motion.as_mut()
    {
        let target = ground_to_field * *ball_position + vector![-1.0, 5.0];
        *target_speed = 1.0;
        *soft = false;
        *quick = true;
        *strong = false;
        return apply_kick_target(blackboard, target);
    }
    Status::Failure
}

pub fn apply_kick_target(
    blackboard: &mut Blackboard,
    target_position_in_field: Point2<Field>,
) -> Status {
    if let Some(ground_to_field) = blackboard.world_state.robot.ground_to_field
        && let Some(BodyMotion::Kick {
            ball_position,
            kick_direction,
            ..
        }) = blackboard.body_motion.as_mut()
    {
        let target_position = ground_to_field.inverse() * target_position_in_field;
        *kick_direction = Orientation2::from_vector(target_position - *ball_position);
        blackboard.kick_target = Some(target_position);
        if blackboard.last_motion_type != Some(MotionType::Kick) {
            blackboard.last_kick_target = Some(target_position_in_field);
        }
        return Status::Success;
    }

    Status::Failure
}

pub fn kick_strength_subtree() -> Node<Blackboard> {
    selection!(
        sequence!(
            condition!(is_last_motion_type, MotionType::Kick),
            action!(use_last_kick_settings)
        ),
        sequence!(
            condition!(is_target_in_strong_kick_range),
            condition!(allow_strong_kicks),
            action!(use_strong_kick)
        ),
        action!(disable_strong_kick)
    )
}

pub fn is_target_in_strong_kick_range(blackboard: &mut Blackboard) -> bool {
    if let Some(target_position) = blackboard.kick_target
        && let Some(BodyMotion::Kick { ball_position, .. }) = &blackboard.body_motion
    {
        if let Some(ground_to_field) = blackboard.world_state.robot.ground_to_field {
            let field = &blackboard.field_dimensions;
            let target = ground_to_field * target_position;
            // Field/Ground roundoff must not turn a goal-line target into a field target.
            let tolerance = 1e-5;
            if target.x() >= field.length / 2.0 - tolerance
                && target.y().abs() <= field.goal_inner_width / 2.0 + tolerance
            {
                return goal_mouth(field, Half::Opponent)
                    .distance_to(ground_to_field * *ball_position)
                    > blackboard
                        .parameters
                        .kicking
                        .strong_kick_min_target_distance;
            }
        }
        (target_position - *ball_position).norm()
            >= blackboard
                .parameters
                .kicking
                .strong_kick_min_target_distance
    } else {
        false
    }
}

pub fn allow_strong_kicks(blackboard: &mut Blackboard) -> bool {
    blackboard.parameters.kicking.allow_strong_kicks
}

pub fn use_last_kick_settings(blackboard: &mut Blackboard) -> Status {
    if let Some(MotionCommand::Kick {
        strong: last_strong,
        soft: last_soft,
        quick: last_quick,
        target_speed: last_target_speed,
        ..
    }) = blackboard.last_motion_command
        && let Some(BodyMotion::Kick {
            strong: motion_strong,
            soft,
            quick,
            target_speed,
            ..
        }) = blackboard.body_motion.as_mut()
    {
        *motion_strong = last_strong;
        *soft = last_soft;
        *quick = last_quick;
        *target_speed = last_target_speed;

        return Status::Success;
    }
    Status::Failure
}

pub fn use_kick(blackboard: &mut Blackboard, strong: bool) -> Status {
    if let Some(BodyMotion::Kick {
        strong: motion_strong,
        ..
    }) = blackboard.body_motion.as_mut()
    {
        *motion_strong = strong;

        return Status::Success;
    }
    Status::Failure
}

pub fn use_strong_kick(blackboard: &mut Blackboard) -> Status {
    use_kick(blackboard, true)
}

pub fn disable_strong_kick(blackboard: &mut Blackboard) -> Status {
    use_kick(blackboard, false)
}

pub fn intercept(blackboard: &mut Blackboard) -> Status {
    if let Some(BodyMotion::Kick {
        ball_position,
        ball_velocity,
        kick_direction: motion_kick_direction,
        strong,
        soft,
        ..
    }) = blackboard.body_motion.as_mut()
    {
        if ball_velocity.norm() < f32::EPSILON {
            return Status::Failure;
        }
        let time_to_closest_approach =
            -(*ball_position).coords().dot(ball_velocity) / ball_velocity.norm_squared();
        if time_to_closest_approach < 0.0 {
            return Status::Failure;
        }

        let interception_point = *ball_position + *ball_velocity * time_to_closest_approach;
        if interception_point.x()
            < blackboard
                .parameters
                .kicking
                .minimum_interception_forward_distance
        {
            return Status::Failure;
        }

        if interception_point.coords().norm()
            > blackboard
                .parameters
                .ball
                .interception
                .maximum_intercept_distance
        {
            return Status::Failure;
        }

        let kick_direction = Orientation2::from_vector(*ball_position - interception_point);

        // The policy needs the observed position and velocity together.
        // Use the predicted interception point only to choose kick direction.
        blackboard.kick_target = Some(*ball_position);
        *motion_kick_direction = kick_direction;
        *strong = false;
        *soft = false;
        return Status::Success;
    }
    Status::Failure
}
pub fn set_kick_target_beyond_ball(blackboard: &mut Blackboard) -> Status {
    if let Some(ground_to_field) = blackboard.world_state.robot.ground_to_field
        && let Some(ball_in_ground) = kick_ball_position_in_ground(blackboard, &ground_to_field)
        && let Some(BodyMotion::Kick {
            kick_direction: motion_kick_direction,
            ..
        }) = blackboard.body_motion.as_mut()
    {
        if blackboard.last_motion_type != Some(MotionType::Kick) {
            let Some(direction) = ball_in_ground.coords().try_normalize(f32::EPSILON) else {
                return Status::Failure;
            };
            let kick_target = ground_to_field * (ball_in_ground + direction * 10.0);
            blackboard.last_kick_target = Some(kick_target);
        }

        if let Some(target_in_field) = blackboard.last_kick_target {
            let field_to_ground = ground_to_field.inverse();
            let target_position = field_to_ground * target_in_field;
            let kick_direction = Orientation2::from_vector(target_position - ball_in_ground);

            blackboard.kick_target = Some(target_position);
            *motion_kick_direction = kick_direction;

            return Status::Success;
        }
    }
    Status::Failure
}

fn visual_kick_ball_in_ground(
    blackboard: &Blackboard,
    ground_to_field: &Isometry2<Ground, Field>,
) -> Option<(Point2<Ground>, Vector2<Ground>)> {
    blackboard
        .visual_kick_ball_position
        .as_ref()
        .map(|ball| (ball.position, ball.velocity))
        .or_else(|| kick_ball_in_ground(blackboard, ground_to_field))
        .filter(|(position, velocity)| {
            position
                .inner
                .iter()
                .chain(velocity.inner.iter())
                .all(|value| value.is_finite())
        })
}

pub(crate) fn kick_ball_position_in_ground(
    blackboard: &Blackboard,
    ground_to_field: &Isometry2<Ground, Field>,
) -> Option<Point2<Ground>> {
    visual_kick_ball_in_ground(blackboard, ground_to_field).map(|(position, _)| position)
}

pub(crate) fn kick_ball_in_ground(
    blackboard: &Blackboard,
    ground_to_field: &Isometry2<Ground, Field>,
) -> Option<(Point2<Ground>, Vector2<Ground>)> {
    // Keep the observed position and velocity in their shared Ground frame.
    blackboard
        .world_state
        .ball
        .map(|ball| (ball.ball_in_ground, ball.ball_in_ground_velocity))
        .or_else(|| {
            let ball = blackboard.ball.as_ref()?;
            let field_to_ground = ground_to_field.inverse();
            Some((
                field_to_ground * ball.position,
                field_to_ground * ball.velocity,
            ))
        })
        .filter(|(position, velocity)| {
            position
                .inner
                .iter()
                .chain(velocity.inner.iter())
                .all(|value| value.is_finite())
        })
}

#[cfg(test)]
mod observation_tests {
    use linear_algebra::vector;
    use types::{
        ball_position::BallPosition,
        field_dimensions::Side,
        world_state::{BallSource, BallState},
    };

    use crate::node::LastBall;

    use super::*;

    #[test]
    fn direct_kick_preserves_visual_ball_pair_despite_cached_pose_and_world_disagreement() {
        let mut board = Blackboard {
            ball: Some(LastBall {
                position: point![2.0, 1.2],
                velocity: vector![0.0, -1.5],
                age: Default::default(),
                field_side: Side::Left,
                source: BallSource::Own,
            }),
            ..Default::default()
        };
        board.world_state.ball = Some(BallState {
            ball_in_ground: point![0.6, 0.1],
            ball_in_ground_velocity: vector![-1.5, 0.3],
            ..BallState::new_at_center(Default::default())
        });
        board.visual_kick_ball_position = Some(BallPosition {
            position: point![0.4, 0.1],
            velocity: vector![-0.8, 0.2],
            last_seen: Default::default(),
        });
        for transform in [
            Isometry2::identity(),
            Isometry2::from_parts(vector![1.0, 2.0], std::f32::consts::FRAC_PI_2),
            Isometry2::from_parts(vector![-0.5, 0.3], -std::f32::consts::FRAC_PI_2),
        ] {
            board.world_state.robot.ground_to_field = Some(transform);
            assert_eq!(kick(&mut board), Status::Success);
            assert!(matches!(board.body_motion,
                Some(BodyMotion::Kick { ball_position, ball_velocity, .. })
                if ball_position == point![0.4, 0.1] && ball_velocity == vector![-0.8, 0.2]));
        }
    }

    #[test]
    fn cached_ball_rebases_position_and_velocity_together_without_lookahead() {
        let board = Blackboard {
            ball: Some(LastBall {
                position: point![1.0, 2.0],
                velocity: vector![3.0, 4.0],
                age: Default::default(),
                field_side: Side::Left,
                source: BallSource::Own,
            }),
            ..Default::default()
        };
        for (angle, expected_position, expected_velocity) in [
            (
                std::f32::consts::FRAC_PI_2,
                point![2.0, 0.0],
                vector![4.0, -3.0],
            ),
            (
                -std::f32::consts::FRAC_PI_2,
                point![-2.0, 0.0],
                vector![-4.0, 3.0],
            ),
        ] {
            let transform = Isometry2::from_parts(vector![1.0, 0.0], angle);
            let (position, velocity) = kick_ball_in_ground(&board, &transform).unwrap();
            assert!((position - expected_position).norm() < 1e-6);
            assert!((velocity - expected_velocity).norm() < 1e-6);
        }
    }

    #[test]
    fn invalid_observed_velocity_does_not_become_a_stationary_kick() {
        let mut board = Blackboard::default();
        board.world_state.ball = Some(BallState {
            ball_in_ground: point![0.6, 0.1],
            ball_in_ground_velocity: vector![f32::NAN, 0.0],
            ..BallState::new_at_center(Default::default())
        });
        assert!(kick_ball_in_ground(&board, &Isometry2::identity()).is_none());
    }

    #[test]
    fn interception_uses_regular_kick_after_a_strong_or_soft_command() {
        let mut blackboard = Blackboard {
            body_motion: Some(BodyMotion::Kick {
                ball_position: point![1.0, 1.0],
                ball_velocity: vector![-1.0, -2.0],
                kick_direction: Default::default(),
                target_speed: 3.4,
                strong: true,
                soft: true,
                quick: false,
            }),
            ..Default::default()
        };
        blackboard
            .parameters
            .ball
            .interception
            .maximum_intercept_distance = 1.5;
        blackboard
            .parameters
            .kicking
            .minimum_interception_forward_distance = 0.3;
        assert_eq!(intercept(&mut blackboard), Status::Success);
        assert!(matches!(
            blackboard.body_motion,
            Some(BodyMotion::Kick {
                strong: false,
                soft: false,
                target_speed: 3.4,
                ..
            })
        ));
    }
}
