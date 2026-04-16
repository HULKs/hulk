use linear_algebra::vector;
use types::{
    behavior_tree::Status,
    motion_command::{HeadMotion, ImageRegion, MotionCommand},
};

use crate::behavior::node::Blackboard;

pub fn injected_motion_command(blackboard: &mut Blackboard) -> Status {
    if let Some(injected_motion_command) = &blackboard.parameters.injected_motion_command {
        blackboard.motion = Some(injected_motion_command.clone());
        Status::Success
    } else {
        Status::Failure
    }
}

pub fn leuchtturm(blackboard: &mut Blackboard) -> Status {
    blackboard.motion = Some(MotionCommand::WalkWithVelocity {
        head: HeadMotion::SearchForLostBall,
        velocity: vector!(0.0, 0.0),
        angular_velocity: 1.0,
    });
    Status::Success
}

pub fn prepare(blackboard: &mut Blackboard) -> Status {
    blackboard.motion = Some(MotionCommand::Prepare);
    Status::Success
}

pub fn stand(blackboard: &mut Blackboard) -> Status {
    blackboard.motion = Some(MotionCommand::Stand {
        head: HeadMotion::Center {
            image_region_target: ImageRegion::Top,
        },
    });
    Status::Success
}

pub fn stand_up(blackboard: &mut Blackboard) -> Status {
    blackboard.motion = Some(MotionCommand::StandUp);
    Status::Success
}
