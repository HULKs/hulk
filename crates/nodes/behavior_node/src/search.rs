use linear_algebra::{Pose2, vector};
use types::{
    behavior_tree::Status,
    motion_command::{BodyMotion, MotionCommand, OrientationMode},
};

use crate::{node::Blackboard, walk::walk_to};

pub fn has_suggested_search_position(blackboard: &mut Blackboard) -> bool {
    blackboard.world_state.suggested_search_position.is_some()
}

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

pub fn walk_to_search_position(blackboard: &mut Blackboard) -> Status {
    if let (Some(search_position), Some(ground_to_field)) = (
        blackboard.world_state.suggested_search_position,
        blackboard.world_state.robot.ground_to_field,
    ) {
        let search_position_in_ground = ground_to_field.inverse() * search_position;

        return walk_to(
            blackboard,
            Pose2::from(search_position_in_ground),
            blackboard.parameters.walk_speed.search,
            OrientationMode::AlignWithPath,
            blackboard
                .parameters
                .walk_and_stand
                .normal_distance_to_be_aligned,
            blackboard.parameters.walk_and_stand.hysteresis,
        );
    }

    Status::Failure
}
