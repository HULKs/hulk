use types::{MotionCommand, PrimaryState, WorldState};

pub fn execute(world_state: &WorldState) -> Option<MotionCommand> {
    match world_state.robot.primary_state {
        PrimaryState::Calibration => Some(MotionCommand::Stand {
            head: types::HeadMotion::Unstiff,
            is_energy_saving: false,
        }),
        _ => None,
    }
}
