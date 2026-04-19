use color_eyre::eyre::{Result, eyre};
use types::{
    behavior_tree::Status,
    motion_command::{BodyMotion, HeadMotion, ImageRegion, MotionCommand},
};

use crate::behavior::node::Blackboard;

pub fn assemble_motion_command(blackboard: &Blackboard, status: Status) -> Result<MotionCommand> {
    match status {
        Status::Success => {
            if blackboard.is_injected_motion_command {
                if let Some(injected_motion_command) =
                    &blackboard.parameters.injected_motion_command
                {
                    return Ok(injected_motion_command.clone());
                }
            }
            let head = if let Some(head_motion) = &blackboard.head_motion {
                *head_motion
            } else {
                HeadMotion::Center {
                    image_region_target: ImageRegion::Center,
                }
            };
            let body = if let Some(body_motion) = &blackboard.body_motion {
                body_motion.clone()
            } else {
                BodyMotion::Stand
            };
            Ok(MotionCommand::from_partial_motions(body, head))
        }
        Status::Failure => Ok(MotionCommand::Stand {
            head: HeadMotion::Center {
                image_region_target: ImageRegion::Center,
            },
        }),
        Status::Idle => {
            Err(eyre!(
                "Behavior tree returned Idle status, which should not happen during a cycle",
            ))
        }
    }
}
