use types::{
    behavior_tree::Status,
    motion_command::{HeadMotion, ImageRegion, MotionCommand},
};

use crate::behavior::node::Blackboard;

pub fn injected_motion_command(blackboard: &mut Blackboard) -> Status {
    if let Some(injected_motion_command) = &blackboard.parameters.injected_motion_command {
        blackboard.output = Some(injected_motion_command.clone());
        Status::Success
    } else {
        Status::Failure
    }
}

pub fn prepare(blackboard: &mut Blackboard) -> Status {
    blackboard.output = Some(MotionCommand::Prepare);
    Status::Success
}

pub fn stand(blackboard: &mut Blackboard) -> Status {
    blackboard.output = Some(MotionCommand::Stand {
        head: HeadMotion::Center {
            image_region_target: ImageRegion::Top,
        },
    });
    Status::Success
}

pub fn stand_up(blackboard: &mut Blackboard) -> Status {
    blackboard.output = Some(MotionCommand::StandUp);
    Status::Success
}

pub fn walk_to_ball(blackboard: &mut Blackboard) -> Status {
    if let Some(ball) = &blackboard.world_state.ball {
        blackboard.output = Some(MotionCommand::WalkWithVelocity {
            head: HeadMotion::LookAt {
                target: ball.ball_in_ground,
                image_region_target: ImageRegion::Top,
            },
            velocity: ball.ball_in_ground.coords(),
            angular_velocity: 0.0,
        });
        Status::Success
    } else {
        Status::Failure
    }
}
