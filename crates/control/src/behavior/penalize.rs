use types::{MotionCommand, PrimaryState, WorldState};

pub fn execute(world_state: &WorldState) -> Option<MotionCommand> {
    match world_state.robot.primary_state {
        PrimaryState::Penalized | PrimaryState::Initial => Some(MotionCommand::Penalized),
        _ => None,
    }
}
