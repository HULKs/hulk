use color_eyre::eyre::{Result, eyre};
use types::{
    behavior_command::{BehaviorCommand, BodyMotion, HeadMotion, ImageRegion},
    behavior_tree::Status,
};

use crate::node::Blackboard;

pub fn assemble_behavior_command(
    blackboard: &Blackboard,
    status: Status,
) -> Result<BehaviorCommand> {
    match status {
        Status::Success => {
            if blackboard.is_injected_behavior_command
                && let Some(injected_behavior_command) =
                    &blackboard.parameters.control.injected_behavior_command
            {
                return Ok(injected_behavior_command.clone());
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
            Ok(BehaviorCommand::from_partial_motions(body, head))
        }
        Status::Failure => Ok(BehaviorCommand::Stand {
            head: HeadMotion::Center {
                image_region_target: ImageRegion::Center,
            },
        }),
        Status::Idle => Err(eyre!(
            "Behavior tree returned Idle status, which should not happen during a cycle",
        )),
    }
}
