use linear_algebra::vector;
use types::{
    behavior_tree::Status,
    controller_input::{Axis, Button},
    motion_command::{BodyMotion, HeadMotion, MotionCommand},
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
            input.axis_value(Axis::LeftStickY),
            -input.axis_value(Axis::LeftStickX)
        ],
        angular_velocity: -input.axis_value(Axis::RightStickX),
    });
    blackboard.head_motion = Some(HeadMotion::MoveWithVelocity {
        yaw: input.button_value(Button::DPadLeft) - input.button_value(Button::DPadRight),
        pitch: input.button_value(Button::DPadDown) - input.button_value(Button::DPadUp),
    });
    Status::Success
}

pub fn stand(blackboard: &mut Blackboard) -> Status {
    blackboard.body_motion = Some(BodyMotion::Stand);
    Status::Success
}

pub fn stand_up(blackboard: &mut Blackboard) -> Status {
    let fast = match blackboard.last_motion_command {
        MotionCommand::StandUp { fast } => fast,
        _ => blackboard.parameters.stand_up.fast,
    };
    blackboard.body_motion = Some(BodyMotion::StandUp { fast });
    Status::Success
}
