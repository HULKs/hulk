use crate::types::{FallState, MotionCommand, WorldState};

pub fn execute(world_state: &WorldState) -> Option<MotionCommand> {
    match world_state.robot.fall_state {
        FallState::Falling { direction } => Some(MotionCommand::FallProtection { direction }),
        _ => None,
    }
}
