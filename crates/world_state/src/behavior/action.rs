use types::motion_command::{HeadMotion, ImageRegion, MotionCommand};

use crate::behavior::{new_behavior::behavior_tree::Status, node::CaptainBlackboard};

pub fn stand(context: &mut CaptainBlackboard) -> Status {
    context.output = Some(MotionCommand::Stand {
        head: HeadMotion::LookAround,
    });
    Status::Success
}

pub fn walk_to_ball(context: &mut CaptainBlackboard) -> Status {
    if let Some(ball) = &context.world_state.ball {
        context.output = Some(MotionCommand::WalkWithVelocity {
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
