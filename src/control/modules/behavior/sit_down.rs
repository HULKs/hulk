use crate::types::{HeadMotion, MotionCommand, PrimaryState, WorldState};

pub fn execute(world_state: &WorldState) -> Option<MotionCommand> {
    match world_state.robot.primary_state {
        PrimaryState::Finished => Some(MotionCommand::SitDown {
            head: HeadMotion::Unstiff,
        }),
        _ => None,
    }
}
