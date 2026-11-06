use color_eyre::eyre::{Result, eyre};
use types::{
    audio::{Sound, Soundboard},
    behavior_tree::Status,
    motion_command::{BodyMotion, HeadMotion, ImageRegion, MotionCommand},
    walking_velocity_limits::WalkingVelocityLimits,
};

use crate::node::Blackboard;

pub(crate) fn sound_for_motion_transition(
    soundboard: &Soundboard,
    previous: Option<&MotionCommand>,
    current: &MotionCommand,
) -> Option<Sound> {
    match (previous, current) {
        (Some(previous), current)
            if soundboard.drift
                && matches!(
                    previous,
                    MotionCommand::Walk { .. } | MotionCommand::WalkWithVelocity { .. }
                )
                && !is_stationary_command(previous)
                && is_stationary_command(current) =>
        {
            Some(Sound::Drift)
        }
        _ => None,
    }
}

fn is_stationary_command(command: &MotionCommand) -> bool {
    match command {
        MotionCommand::Stand { .. } => true,
        MotionCommand::WalkWithVelocity {
            velocity,
            angular_velocity,
            ..
        } => velocity.x() == 0.0 && velocity.y() == 0.0 && *angular_velocity == 0.0,
        _ => false,
    }
}

pub fn assemble_motion_command(blackboard: &Blackboard, status: Status) -> Result<MotionCommand> {
    let command = match status {
        Status::Success => {
            if blackboard.is_injected_motion_command
                && let Some(injected_motion_command) =
                    &blackboard.parameters.control.injected_motion_command
            {
                injected_motion_command.clone()
            } else {
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
                MotionCommand::from_partial_motions(body, head)
            }
        }
        Status::Failure => MotionCommand::Stand {
            head: HeadMotion::Center {
                image_region_target: ImageRegion::Center,
            },
        },
        Status::Idle => {
            return Err(eyre!(
                "Behavior tree returned Idle status, which should not happen during a cycle",
            ));
        }
    };

    Ok(clamp_walking_velocity(
        command,
        blackboard.walking_velocity_limits,
    ))
}

fn clamp_walking_velocity(command: MotionCommand, limits: WalkingVelocityLimits) -> MotionCommand {
    match command {
        MotionCommand::WalkWithVelocity {
            head,
            velocity,
            angular_velocity,
        } => {
            let (velocity, angular_velocity) = limits.clamp_command(velocity, angular_velocity);
            MotionCommand::WalkWithVelocity {
                head,
                velocity,
                angular_velocity,
            }
        }
        command => command,
    }
}
