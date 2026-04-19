use coordinate_systems::Field;
use geometry::line::Line;
use linear_algebra::{Orientation2, Point, Rotation2};
use types::{
    behavior_tree::Status,
    motion_command::{BodyMotion, KickPower},
};

use crate::behavior::node::Blackboard;

pub fn kick(blackboard: &mut Blackboard) -> Status {
    if let (Some(ball), Some(ground_to_field), Some(kick_target)) = (
        &blackboard.ball,
        &blackboard.world_state.robot.ground_to_field,
        &blackboard.kick_target,
    ) {
        let ball_in_ground = ground_to_field.inverse() * ball.position;
        let parameters = &blackboard.parameters.kicking;

        let robot_theta_to_field: Orientation2<Field> = ground_to_field.orientation();

        blackboard.body_motion = Some(BodyMotion::VisualKick {
            ball_position: ball_in_ground,
            kick_direction: kick_target.direction,
            target_position: Rotation2::new(parameters.kick_target_offset_angle)
                * kick_target.position,
            robot_theta_to_field,
            kick_power: blackboard
                .last_kick_power
                .unwrap_or(KickPower::Rumpelstilzchen),
        });

        Status::Success
    } else {
        Status::Failure
    }
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

        let robot_theta_to_field: Orientation2<Field> = ground_to_field.orientation();

        let ball_position = ball_in_ground;

        let kick_direction =
            Orientation2::from_vector(ball_position.coords() - interception_point.coords());

        blackboard.body_motion = Some(BodyMotion::VisualKick {
            ball_position: interception_point,
            kick_direction,
            target_position: ball_position,
            robot_theta_to_field,
            kick_power: blackboard
                .last_kick_power
                .unwrap_or(KickPower::Rumpelstilzchen),
        });
        Status::Success
    } else {
        Status::Failure
    }
}

pub fn kick_instead_of_walking(blackboard: &mut Blackboard) -> Status {
    if let (Some(ball), Some(ground_to_field), Some(kick_target)) = (
        &blackboard.last_ball,
        &blackboard.world_state.robot.ground_to_field,
        &blackboard.kick_target,
    ) {
        blackboard.is_alternative_kick = true;

        let field_to_ground = ground_to_field.inverse();
        let ball_in_ground = field_to_ground * ball.position;

        let robot_theta_to_field: Orientation2<Field> = ground_to_field.orientation();

        blackboard.body_motion = Some(BodyMotion::VisualKick {
            ball_position: ball_in_ground,
            kick_direction: kick_target.direction,
            target_position: Rotation2::new(blackboard.parameters.kicking.kick_target_offset_angle)
                * kick_target.position,
            robot_theta_to_field,
            kick_power: blackboard
                .last_kick_power
                .unwrap_or(KickPower::Rumpelstilzchen),
        });
        Status::Success
    } else {
        Status::Failure
    }
}
