use linear_algebra::vector;
use types::{
    behavior_tree::Status,
    motion_command::{BodyMotion, HeadMotion},
};

use crate::node::Blackboard;

pub fn damping(blackboard: &mut Blackboard) -> Status {
    blackboard.body_motion = Some(BodyMotion::Damping);
    Status::Success
}

pub fn injected_motion_command(blackboard: &mut Blackboard) -> Status {
    if blackboard
        .parameters
        .control
        .injected_motion_command
        .is_some()
    {
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

pub fn remote_control(blackboard: &mut Blackboard) -> Status {
    let Some(input) = &blackboard.controller_input else {
        return Status::Failure;
    };

    blackboard.body_motion = Some(BodyMotion::WalkWithVelocity {
        velocity: vector![
            input.axis_value("LeftStickY"),
            -input.axis_value("LeftStickX")
        ],
        angular_velocity: -input.axis_value("RightStickX"),
    });
    blackboard.head_motion = Some(HeadMotion::MoveWithVelocity {
        yaw: input.button_value("DPadLeft") - input.button_value("DPadRight"),
        pitch: input.button_value("DPadDown") - input.button_value("DPadUp"),
    });
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
