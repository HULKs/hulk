use types::{
    motion_command::MotionCommand,
    primary_state::PrimaryState,
    world_state::WorldState,
};

pub fn execute(world_state: &WorldState) -> Option<MotionCommand> {
    if world_state.robot.primary_state != PrimaryState::Initial {
        return None;
    }

    Some(MotionCommand::Prepare)
}
