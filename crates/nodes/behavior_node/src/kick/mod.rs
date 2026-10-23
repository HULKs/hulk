pub mod execution;
pub mod geometry;
pub mod kickoff;
pub mod passing;
pub mod reception;
pub mod strategy;

use ::geometry::line_segment::LineSegment;
use coordinate_systems::{Field, Ground};
use hsl_network_messages::PlayerNumber;
use linear_algebra::{Isometry2, Orientation2, Point2, Vector2, point};
use types::{
    behavior_tree::Status,
    motion_command::{BodyMotion, HeadMotion, ImageRegion, MotionCommand},
    motion_type::MotionType,
};

use crate::{
    action,
    actions::stand,
    behavior_tree::Node,
    condition,
    node::Blackboard,
    selection, sequence, subtree,
    switch_motion_type::{is_last_motion_type, switch_motion_type},
    walk::walk_to_ball,
};

pub(crate) const RESTART_RECEIVER_EXTRA_DISTANCE: f32 = 0.5;

pub fn kick_subtree() -> Node<Blackboard> {
    switch_motion_type(
        MotionType::Kick,
        sequence!(
            action!(kick),
            action!(select_kick_target),
            subtree!(kick_strength_subtree),
        ),
        subtree!(kick_alternatives_subtree),
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
    blackboard.ball_action = None;
    blackboard.kick_target = None;
    if let Some(ground_to_field) = &blackboard.world_state.robot.ground_to_field
        && let Some((ball_in_ground, ball_velocity)) =
            kick_ball_in_ground(blackboard, ground_to_field)
    {
        blackboard.body_motion = Some(BodyMotion::Kick {
            ball_position: ball_in_ground,
            ball_velocity,
            target_speed: blackboard.parameters.kicking.target_speed,
            soft: false,
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
    let field = &blackboard.field_dimensions;
    blackboard.ball_action = Some(strategy::BallAction::Shoot {
        opening: LineSegment(
            point![field.length / 2.0, -field.goal_inner_width / 2.0],
            point![field.length / 2.0, field.goal_inner_width / 2.0],
        ),
    });
    let Some(target) = execution::resolve_target(blackboard) else {
        blackboard.ball_action = None;
        blackboard.kick_target = None;
        stand(blackboard);
        return Status::Failure;
    };
    apply_kick_target(blackboard, target)
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
        return Status::Success;
    }

    Status::Failure
}

pub fn kick_strength_subtree() -> Node<Blackboard> {
    // Shots and long clearances must not inherit a preceding interception's weaker flag.
    selection!(
        sequence!(
            condition!(is_target_in_strong_kick_range),
            condition!(allow_strong_kicks),
            action!(use_strong_kick)
        ),
        sequence!(
            condition!(is_last_motion_type, MotionType::Kick),
            action!(use_last_kick_settings)
        ),
        action!(disable_strong_kick)
    )
}

pub fn is_target_in_strong_kick_range(blackboard: &mut Blackboard) -> bool {
    if let Some(target_position) = blackboard.kick_target
        && let Some(BodyMotion::Kick { ball_position, .. }) = &blackboard.body_motion
    {
        matches!(
            blackboard.ball_action,
            Some(strategy::BallAction::Shoot { .. } | strategy::BallAction::ShootAtGoalCenter)
        ) || (target_position - *ball_position).norm()
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
        *soft = false;
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

pub(crate) fn kick_ball_position_in_ground(
    blackboard: &Blackboard,
    ground_to_field: &Isometry2<Ground, Field>,
) -> Option<Point2<Ground>> {
    kick_ball_in_ground(blackboard, ground_to_field).map(|(position, _)| position)
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

pub(super) fn pass_receiver_position(
    blackboard: &Blackboard,
    ball: Point2<Field>,
    receiver: PlayerNumber,
) -> Option<Point2<Field>> {
    let world = &blackboard.world_state;
    let parameters = &blackboard.parameters.kicking.strategy.passing;
    if receiver == blackboard.player_number
        || receiver == blackboard.parameters.goalkeeper.player_number
        || Some(receiver) == blackboard.replacement_goalkeeper_coordinator.keeper()
        || world
            .filtered_game_controller_state
            .as_ref()
            .is_some_and(|state| state.penalties[receiver].is_some())
    {
        return None;
    }
    let state = world.player_states[receiver].as_ref()?;
    if state.time > world.now
        || world.now.duration_since(state.time) >= parameters.maximum_receiver_age
    {
        return None;
    }
    let position = state.inner.pose.position();
    let field = &blackboard.field_dimensions;
    (position
        .inner
        .iter()
        .all(|coordinate| coordinate.is_finite())
        && position.x().abs() < field.length / 2.0
        && position.y().abs() < field.width / 2.0
        && (position.x() - ball.x() >= parameters.minimum_progress
            || passing::is_executing_announced_pass(blackboard, receiver)
            || world
                .filtered_game_controller_state
                .as_ref()
                .is_some_and(|game| {
                    game.kicking_team == Some(hsl_network_messages::Team::Hulks)
                        && game.sub_state.is_some_and(|state| {
                            state != hsl_network_messages::SubState::PenaltyKick
                        })
                })))
    .then_some(position)
}

#[cfg(test)]
mod tests {
    use linear_algebra::vector;
    use types::{
        ball_position::BallPosition,
        field_dimensions::{FieldDimensions, Side},
        obstacles::Obstacle,
        world_state::{BallSource, BallState},
    };

    use crate::node::LastBall;

    use super::*;

    #[test]
    fn kick_preserves_world_ball_pair_despite_cached_pose_and_visual_disagreement() {
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
            velocity: Vector2::zeros(),
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
                if ball_position == point![0.6, 0.1] && ball_velocity == vector![-1.5, 0.3]));
        }
    }

    #[test]
    fn cached_ball_rebases_position_and_velocity_together_without_lookahead() {
        let mut board = Blackboard::default();
        board.parameters.ball.pursuit_lookahead = std::time::Duration::from_secs(1);
        board.ball = Some(LastBall {
            position: point![1.0, 2.0],
            velocity: vector![3.0, 4.0],
            age: Default::default(),
            field_side: Side::Left,
            source: BallSource::Own,
        });
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
    fn setplay_shots_choose_clear_goal_opening_and_stop_when_blocked() {
        let mut blackboard = Blackboard {
            field_dimensions: FieldDimensions::SPL_2025,
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
        blackboard.parameters.kicking.strategy.maximum_shot_distance = 12.0;
        blackboard.parameters.kicking.strategy.obstacle_clearance = 0.05;
        blackboard.world_state.obstacles = vec![Obstacle::robot(point![3.0, 0.0], 0.2, 0.2)];
        assert_eq!(kick(&mut blackboard), Status::Success);
        assert_eq!(select_kick_target(&mut blackboard), Status::Success);
        assert!(blackboard.kick_target.unwrap().y().abs() > 0.1);

        blackboard.world_state.obstacles[0].radius_at_foot_height = 1.0;
        assert_eq!(kick(&mut blackboard), Status::Success);
        assert_eq!(select_kick_target(&mut blackboard), Status::Failure);
        assert_eq!(blackboard.body_motion, Some(BodyMotion::Stand));
        assert_eq!(blackboard.kick_target, None);
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

    #[test]
    fn legacy_goal_shots_use_strong_kicks_even_at_short_range() {
        let mut board = Blackboard {
            kick_target: Some(point![1.0, 0.0]),
            last_motion_type: Some(MotionType::Kick),
            body_motion: Some(BodyMotion::Kick {
                ball_position: point![0.5, 0.0],
                ball_velocity: Vector2::zeros(),
                kick_direction: Default::default(),
                target_speed: 3.4,
                strong: false,
                soft: false,
                quick: false,
            }),
            ..Default::default()
        };
        board.parameters.kicking.allow_strong_kicks = true;
        board.parameters.kicking.strong_kick_min_target_distance = 3.0;
        for action in [
            strategy::BallAction::ShootAtGoalCenter,
            strategy::BallAction::Shoot {
                opening: LineSegment(point![1.0, -0.5], point![1.0, 0.5]),
            },
        ] {
            board.ball_action = Some(action);
            disable_strong_kick(&mut board);
            assert_eq!(
                kick_strength_subtree().tick_with_trace(&mut board).0,
                Status::Success
            );
            assert!(matches!(
                board.body_motion,
                Some(BodyMotion::Kick { strong: true, .. })
            ));
        }
        board.ball_action = Some(strategy::BallAction::Dribble);
        assert!(!is_target_in_strong_kick_range(&mut board));
    }
}
