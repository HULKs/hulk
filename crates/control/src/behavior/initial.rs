use types::{motion_command::MotionCommand, primary_state::PrimaryState, world_state::WorldState};

pub fn execute(world_state: &WorldState) -> Option<MotionCommand> {
    match world_state.robot.primary_state {
        PrimaryState::Initial => Some(MotionCommand::Initial),
        _ => None,
    }
}
