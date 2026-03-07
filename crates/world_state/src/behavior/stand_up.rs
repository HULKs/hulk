use types::{motion_command::MotionCommand, primary_state::PrimaryState, world_state::WorldState};

pub fn execute(world_state: &WorldState) -> Option<MotionCommand> {
    if !world_state
        .fall_down_state
        .is_some_and(|fall_down_state| fall_down_state.is_recovery_available)
    {
        return None;
    }

    match world_state.robot.primary_state {
        PrimaryState::Safe | PrimaryState::Stop | PrimaryState::Penalized => None,
        _ => Some(MotionCommand::StandUp),
    }
}
