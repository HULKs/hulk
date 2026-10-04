use linear_algebra::vector;
use types::{
    behavior_tree::Status,
    motion_command::{BodyMotion, MotionCommand},
};

use crate::node::Blackboard;

pub fn leuchtturm(blackboard: &mut Blackboard) -> Status {
    let angular_velocity = get_leuchtturm_direction(blackboard);

    blackboard.body_motion = Some(BodyMotion::WalkWithVelocity {
        velocity: vector!(0.0, 0.0),
        angular_velocity,
    });
    Status::Success
}

fn get_leuchtturm_direction(blackboard: &Blackboard) -> f32 {
    if let MotionCommand::WalkWithVelocity {
        angular_velocity, ..
    } = blackboard.last_motion_command
        && angular_velocity.abs() > f32::EPSILON
    {
        return angular_velocity.signum();
    }

    if let (Some(last_ball), Some(ground_to_field)) = (
        &blackboard.last_ball,
        blackboard.world_state.robot.ground_to_field,
    ) {
        let ball_in_ground = ground_to_field.inverse() * last_ball.position;

        if ball_in_ground.y() < 0.0 {
            return -1.0;
        }
    }

    1.0
}
