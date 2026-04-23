use types::{behavior_tree::Status, motion_command::BodyMotion};

use crate::behavior::node::Blackboard;

pub fn injected_motion_command(blackboard: &mut Blackboard) -> Status {
    if blackboard.parameters.injected_motion_command.is_some() {
        blackboard.is_injected_motion_command = true;
        Status::Success
    } else {
        Status::Failure
    }
}

pub fn prepare(blackboard: &mut Blackboard) -> Status {
    blackboard.body_motion = Some(BodyMotion::Prepare);
    Status::Success
}

pub fn stand(blackboard: &mut Blackboard) -> Status {
    blackboard.body_motion = Some(BodyMotion::Stand);
    Status::Success
}

pub fn stand_up(blackboard: &mut Blackboard) -> Status {
    blackboard.body_motion = Some(BodyMotion::StandUp);
    Status::Success
}
