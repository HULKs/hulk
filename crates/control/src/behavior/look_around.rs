use types::{MotionCommand, PrimaryState, WorldState};

pub fn execute(world_state: &WorldState) -> Option<MotionCommand> {
    match world_state.robot.primary_state {
        PrimaryState::Ready | PrimaryState::Playing => Some(MotionCommand::Stand {
            head: types::HeadMotion::LookAround,
        }),
        _ => None,
    }
}
