use linear_algebra::{Pose2, vector};
use types::{
    behavior_tree::Status,
    motion_command::{BodyMotion, OrientationMode},
};

use crate::behavior::{node::Blackboard, walk::walk_to};

pub fn has_suggested_search_position(blackboard: &mut Blackboard) -> bool {
    blackboard.world_state.suggested_search_position.is_some()
}

pub fn leuchtturm(blackboard: &mut Blackboard) -> Status {
    blackboard.body_motion = Some(BodyMotion::WalkWithVelocity {
        velocity: vector!(0.0, 0.0),
        angular_velocity: 1.0,
    });
    Status::Success
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
