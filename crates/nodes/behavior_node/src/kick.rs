use coordinate_systems::{Field, Ground};
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
    blackboard.kick_target = None;
    if let Some(ground_to_field) = &blackboard.world_state.robot.ground_to_field
        && let Some(ball_in_ground) = kick_ball_position_in_ground(blackboard, ground_to_field)
    {
        blackboard.body_motion = Some(BodyMotion::Kick {
            ball_position: ball_in_ground,
            ball_velocity: blackboard
                .world_state
                .ball
                .map_or(Vector2::zeros(), |ball| ball.ball_in_ground_velocity),
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
    let goal_position: Point2<Field> = point!(blackboard.field_dimensions.length / 2.0, 0.0);

    apply_kick_target(blackboard, goal_position)
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
    if let MotionCommand::Kick {
        strong: last_strong,
        soft: last_soft,
        quick: last_quick,
        target_speed: last_target_speed,
        ..
    } = blackboard.last_motion_command
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
        ..
    }) = &blackboard.body_motion
    {
        let ball_in_ground = *ball_position;
        let velocity = *ball_velocity;
        if velocity.norm() < f32::EPSILON {
            return Status::Failure;
        }
        let time_to_closest_approach =
            -ball_in_ground.coords().dot(&velocity) / velocity.norm_squared();
        if time_to_closest_approach < 0.0 {
            return Status::Failure;
        }

        let interception_point = ball_in_ground + velocity * time_to_closest_approach;
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

        let kick_direction = Orientation2::from_vector(ball_in_ground - interception_point);

        if let Some(BodyMotion::Kick {
            kick_direction: motion_kick_direction,
            ..
        }) = blackboard.body_motion.as_mut()
        {
            // The policy needs the observed position and velocity together.
            // Use the predicted interception point only to choose kick direction.
            blackboard.kick_target = Some(ball_in_ground);
            *motion_kick_direction = kick_direction;
            return Status::Success;
        }
    }
    Status::Failure
}
pub fn set_kick_target_in_front(blackboard: &mut Blackboard) -> Status {
    if let Some(ground_to_field) = blackboard.world_state.robot.ground_to_field
        && let Some(ball_in_ground) = kick_ball_position_in_ground(blackboard, &ground_to_field)
        && let Some(BodyMotion::Kick {
            kick_direction: motion_kick_direction,
            ..
        }) = blackboard.body_motion.as_mut()
    {
        if blackboard.last_motion_type != Some(MotionType::Kick) {
            let kick_target = ground_to_field * point!(3.0, 0.0);
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

fn kick_ball_position_in_ground(
    blackboard: &Blackboard,
    ground_to_field: &Isometry2<Ground, Field>,
) -> Option<Point2<Ground>> {
    blackboard
        .visual_kick_ball_position
        .as_ref()
        .map(|ball| ball.position)
        .or_else(|| {
            blackboard
                .ball
                .as_ref()
                .map(|ball| ground_to_field.inverse() * ball.position)
        })
}
