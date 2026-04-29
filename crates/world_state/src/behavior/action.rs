use linear_algebra::vector;
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

pub fn leuchtturm(blackboard: &mut Blackboard) -> Status {
    blackboard.body_motion = Some(BodyMotion::WalkWithVelocity {
        velocity: vector!(0.0, 0.0),
        angular_velocity: 1.0,
    });
    Status::Success
}

pub fn prepare(blackboard: &mut Blackboard) -> Status {
    blackboard.body_motion = Some(BodyMotion::Prepare);
    Status::Success
}

pub fn remote_control(blackboard: &mut Blackboard) -> Status {
    let parameters = &blackboard.parameters.remote_control;
    let remote_control_motion_command = BodyMotion::WalkWithVelocity {
        velocity: vector![parameters.walk.forward, parameters.walk.left,],
        angular_velocity: parameters.walk.turn,
    };
    blackboard.body_motion = Some(remote_control_motion_command);
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
