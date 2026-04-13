use coordinate_systems::Field;
use geometry::line::Line;
use linear_algebra::{Orientation2, Point, Rotation2, Vector2, vector};
use types::{
    behavior_tree::Status,
    motion_command::{HeadMotion, ImageRegion, KickPower, MotionCommand},
};

use crate::behavior::node::Blackboard;

pub fn kick(blackboard: &mut Blackboard, kick_power: KickPower) -> Status {
    let ball_position = match &blackboard.world_state.ball {
        Some(ball) => ball.ball_in_ground,
        None => {
            return Status::Failure;
        }
    };
    let ground_to_field = match blackboard.world_state.robot.ground_to_field {
        Some(transform) => transform,
        None => return Status::Failure,
    };
    let parameters = &blackboard.parameters.kicking;

    let distance_to_ball = ball_position.coords().norm();
    let head = if distance_to_ball < parameters.distance_to_look_directly_at_the_ball {
        HeadMotion::LookAt {
            target: ball_position,
            image_region_target: ImageRegion::Center,
        }
    } else {
        HeadMotion::LookLeftAndRightOf {
            target: ball_position,
        }
    };

    let goal_position: Vector2<Field> = vector!(blackboard.field_dimensions.length / 2.0, 0.0);
    let field_to_ground = ground_to_field.inverse();
    let kick_direction =
        Orientation2::from_vector(field_to_ground * goal_position - ball_position.coords());

    let robot_theta_to_field: Orientation2<Field> = ground_to_field.orientation();
    let target_position = (field_to_ground * goal_position).as_point();

    blackboard.output = Some(MotionCommand::VisualKick {
        head,
        ball_position,
        kick_direction,
        target_position: Rotation2::new(parameters.kick_target_offset_angle) * target_position,
        robot_theta_to_field,
        kick_power,
    });

    Status::Success
}

pub fn intercept(blackboard: &mut Blackboard) -> Status {
    if let (Some(ball), Some(ground_to_field)) = (
        &blackboard.world_state.ball,
        &blackboard.world_state.robot.ground_to_field,
    ) {
        let velocity = ball.ball_in_ground_velocity;
        if velocity.norm() < f32::EPSILON {
            return Status::Failure;
        }
        let ball_line = Line {
            point: ball.ball_in_ground,
            direction: ball.ball_in_ground_velocity,
        };
        let interception_point = ball_line.closest_point(Point::origin());

        if interception_point.coords().norm()
            > blackboard.parameters.intercept_ball.maximum_intercept_distance
        {
            return Status::Failure;
        }

        let robot_theta_to_field: Orientation2<Field> = ground_to_field.orientation();

        let ball_position = ball.ball_in_ground;
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

        blackboard.output = Some(MotionCommand::VisualKick {
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
