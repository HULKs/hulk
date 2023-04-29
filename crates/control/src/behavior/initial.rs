use types::{HeadMotion, MotionCommand, PrimaryState, WorldState};

pub fn execute(world_state: &WorldState) -> Option<MotionCommand> {
    match world_state.robot.primary_state {
        PrimaryState::Initial => Some(MotionCommand::Stand {
            head: HeadMotion::ZeroAngles,
            is_energy_saving: true,
        }),
        _ => None,
    }
}
