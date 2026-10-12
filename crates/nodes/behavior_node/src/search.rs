use linear_algebra::{Orientation2, Pose2, vector};
use types::{
    behavior_tree::Status,
    motion_command::{BodyMotion, MotionCommand, OrientationMode},
};

use crate::{node::Blackboard, walk::walk_to};

pub fn walk_towards_search_position(blackboard: &mut Blackboard) -> Status {
    let (Some(search_position), Some(ground_to_field)) = (
        blackboard.world_state.suggested_search_position,
        blackboard.world_state.robot.ground_to_field,
    ) else {
        return Status::Failure;
    };

    let target_in_ground = ground_to_field.inverse() * search_position;
    let position_reached_distance = blackboard.parameters.search.position_reached_distance;
    if target_in_ground.coords().norm() <= position_reached_distance.max(f32::EPSILON) {
        return Status::Failure;
    }

    let search_speed = blackboard.parameters.walking.speed.search;
    let walk_and_stand = blackboard.parameters.walking.walk_and_stand;
    walk_to(
        blackboard,
        Pose2::from_parts(
            target_in_ground,
            Orientation2::from_vector(target_in_ground.coords()),
        ),
        search_speed,
        OrientationMode::AlignWithPath,
        walk_and_stand.normal_distance_to_be_aligned,
        walk_and_stand.hysteresis,
    )
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
