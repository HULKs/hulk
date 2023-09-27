use types::{
    motion_command::{HeadMotion::Unstiff, MotionCommand},
    primary_state::PrimaryState,
    world_state::WorldState,
};

pub fn execute(world_state: &WorldState) -> Option<MotionCommand> {
    match world_state.robot.primary_state {
        PrimaryState::Calibration => Some(MotionCommand::Stand {
            head: Unstiff,
            is_energy_saving: false,
        }),
        _ => None,
    }
}
