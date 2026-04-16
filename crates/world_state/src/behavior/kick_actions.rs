use coordinate_systems::Field;
use geometry::line::Line;
use linear_algebra::{Orientation2, Point, Rotation2, Vector2, vector};
use types::{
    behavior_tree::Status,
    motion_command::{HeadMotion, ImageRegion, KickPower, MotionCommand},
};

use crate::behavior::node::Blackboard;

pub fn kick(blackboard: &mut Blackboard, kick_power: KickPower) -> Status {
    if let (Some(ball), Some(ground_to_field)) =
        (&blackboard.ball, &blackboard.world_state.robot.ground_to_field)
    {
        let ball_in_ground = ground_to_field.inverse() * ball.position;
        let parameters = &blackboard.parameters.kicking;

        let distance_to_ball = ball_in_ground.coords().norm();
        let head = if distance_to_ball < parameters.distance_to_look_directly_at_the_ball {
            HeadMotion::LookAt {
                target: ball_in_ground,
                image_region_target: ImageRegion::Center,
            }
        } else {
            HeadMotion::LookLeftAndRightOf {
                target: ball_in_ground,
            }
        };

        let goal_position: Vector2<Field> = vector!(blackboard.field_dimensions.length / 2.0, 0.0);
        let field_to_ground = ground_to_field.inverse();
        let kick_direction =
            Orientation2::from_vector(field_to_ground * goal_position - ball_in_ground.coords());

        let robot_theta_to_field: Orientation2<Field> = ground_to_field.orientation();
        let target_position = (field_to_ground * goal_position).as_point();

        blackboard.motion = Some(MotionCommand::VisualKick {
            head,
            ball_position: ball_in_ground,
            kick_direction,
            target_position: Rotation2::new(parameters.kick_target_offset_angle) * target_position,
            robot_theta_to_field,
            kick_power,
        });

        Status::Success
    } else {
        Status::Failure
    }
}

pub fn intercept(blackboard: &mut Blackboard) -> Status {
    if let (Some(ball), Some(ground_to_field)) =
        (&blackboard.ball, &blackboard.world_state.robot.ground_to_field)
    {
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
            > blackboard.parameters.intercept_ball.maximum_intercept_distance
        {
            return Status::Failure;
        }

        let robot_theta_to_field: Orientation2<Field> = ground_to_field.orientation();

        let ball_position = ball_in_ground;
        let distance_to_ball = ball_position.coords().norm();
        let head = if distance_to_ball
            < blackboard
                .parameters
                .kicking
                .distance_to_look_directly_at_the_ball
        {
            HeadMotion::LookAt {
                target: ball_position,
                image_region_target: ImageRegion::Center,
            }
        } else {
            HeadMotion::LookLeftAndRightOf {
                target: ball_position,
            }
        };

        let kick_direction =
            Orientation2::from_vector(ball_position.coords() - interception_point.coords());

        blackboard.motion = Some(MotionCommand::VisualKick {
            head,
            ball_position: interception_point,
            kick_direction,
            target_position: ball_position,
            robot_theta_to_field,
            kick_power: KickPower::Rumpelstilzchen,
        });
        Status::Success
    } else {
        Status::Failure
    }
}

pub fn kick_instead_of_walking(blackboard: &mut Blackboard) -> Status {
    if let (Some(ball), Some(ground_to_field)) = (
        &blackboard.last_ball,
        blackboard.world_state.robot.ground_to_field,
    ) {
        blackboard.is_alternative_kick = true;

        let field_to_ground = ground_to_field.inverse();
        let ball_in_ground = field_to_ground * ball.position;

        let goal_position: Vector2<Field> = vector!(blackboard.field_dimensions.length / 2.0, 0.0);
        let field_to_ground = ground_to_field.inverse();
        let kick_direction =
            Orientation2::from_vector(field_to_ground * goal_position - ball_in_ground.coords());

        let robot_theta_to_field: Orientation2<Field> = ground_to_field.orientation();
        let target_position = (field_to_ground * goal_position).as_point();

        blackboard.motion = Some(MotionCommand::VisualKick {
            head: HeadMotion::LookAt {
                target: ball_in_ground,
                image_region_target: ImageRegion::Center,
            },
            ball_position: ball_in_ground,
            kick_direction,
            target_position,
            robot_theta_to_field,
            kick_power: KickPower::Rumpelstilzchen,
        });
        Status::Success
    } else {
        Status::Failure
    }
}
