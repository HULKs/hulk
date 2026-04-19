use coordinate_systems::Field;
use geometry::line::Line;
use linear_algebra::{Orientation2, Point, Point2, Rotation2, point};
use types::{
    behavior_tree::Status,
    motion_command::{BodyMotion, KickPower, MotionCommand},
    motion_type::MotionType,
};

use crate::{
    action,
    behavior::{behavior_tree::Node, node::Blackboard, switch_motion_type::is_last_motion_type},
    condition, negation, selection, sequence,
};

pub fn kick(blackboard: &mut Blackboard) -> Status {
    if let (Some(ball), Some(ground_to_field)) = (
        &blackboard.ball,
        &blackboard.world_state.robot.ground_to_field,
    ) {
        let ball_in_ground = ground_to_field.inverse() * ball.position;
        let robot_theta_to_field: Orientation2<Field> = ground_to_field.orientation();

        blackboard.body_motion = Some(BodyMotion::VisualKick {
            ball_position: ball_in_ground,
            kick_direction: Default::default(),
            target_position: Default::default(),
            robot_theta_to_field,
            kick_power: Default::default(),
        });

        Status::Success
    } else {
        Status::Failure
    }
}

pub fn select_kick_target(context: &mut Blackboard) -> Status {
    if let (Some(ground_to_field), Some(ball)) =
        (context.world_state.robot.ground_to_field, &context.ball)
    {
        let goal_position: Point2<Field> = point!(context.field_dimensions.length / 2.0, 0.0);
        let field_to_ground = ground_to_field.inverse();

        let target_position = field_to_ground * goal_position;

        let ball_in_ground = field_to_ground * ball.position;
        let kick_direction = Orientation2::from_vector(target_position - ball_in_ground);

        if let Some(BodyMotion::VisualKick {
            target_position: motion_target_position,
            kick_direction: motion_kick_direction,
            ..
        }) = context.body_motion.as_mut()
        {
            *motion_target_position =
                Rotation2::new(context.parameters.kicking.kick_target_offset_angle)
                    * target_position;
            *motion_kick_direction = kick_direction;

            return Status::Success;
        }
    }
    Status::Failure
}

pub fn kick_power_subtree() -> Node<Blackboard> {
    selection!(
        sequence!(
            condition!(is_last_motion_type, MotionType::Kick),
            action!(use_last_kick_power)
        ),
        sequence!(
            negation!(condition!(is_close_to_target)),
            condition!(allow_schlong),
            action!(use_kick_power, KickPower::Schlong)
        ),
        action!(use_kick_power, KickPower::Rumpelstilzchen)
    )
}

pub fn is_close_to_target(context: &mut Blackboard) -> bool {
    if let Some(BodyMotion::VisualKick {
        target_position, ..
    }) = &context.body_motion
    {
        target_position.coords().norm()
            < context
                .parameters
                .kicking
                .target_distance_kick_power_threshold
    } else {
        false
    }
}

pub fn allow_schlong(context: &mut Blackboard) -> bool {
    context.parameters.kicking.allow_schlong
}

pub fn use_last_kick_power(context: &mut Blackboard) -> Status {
    if let MotionCommand::VisualKick {
        kick_power: last_kick_power,
        ..
    } = context.last_motion_command
    {
        if let Some(BodyMotion::VisualKick {
            kick_power: motion_kick_power,
            ..
        }) = context.body_motion.as_mut()
        {
            *motion_kick_power = last_kick_power;

            return Status::Success;
        }
    }
    Status::Failure
}

pub fn use_kick_power(context: &mut Blackboard, kick_power: KickPower) -> Status {
    if let Some(BodyMotion::VisualKick {
        kick_power: motion_kick_power,
        ..
    }) = context.body_motion.as_mut()
    {
        *motion_kick_power = kick_power;

        return Status::Success;
    }
    Status::Failure
}

pub fn intercept(blackboard: &mut Blackboard) -> Status {
    if let (Some(ball), Some(ground_to_field)) = (
        &blackboard.ball,
        &blackboard.world_state.robot.ground_to_field,
    ) {
        let ball_in_ground = ground_to_field.inverse() * ball.position;
        let velocity = ball.velocity;
        if velocity.norm() < f32::EPSILON {
            return Status::Failure;
        }
        let ball_line = Line {
            point: ball_in_ground,
            direction: velocity,
        };
        let interception_point = ball_line.closest_point(Point::origin());

        if interception_point.coords().norm()
            > blackboard
                .parameters
                .intercept_ball
                .maximum_intercept_distance
        {
            return Status::Failure;
        }

        let kick_direction =
            Orientation2::from_vector(ball_in_ground - interception_point);

        if let Some(BodyMotion::VisualKick {
            ball_position: motion_ball_position,
            target_position: motion_target_position,
            kick_direction: motion_kick_direction,
            ..
        }) = blackboard.body_motion.as_mut()
        {
            *motion_ball_position = interception_point;
            *motion_target_position = ball_in_ground;
            *motion_kick_direction = kick_direction;
            return Status::Success;
        }
    }
    Status::Failure
}
