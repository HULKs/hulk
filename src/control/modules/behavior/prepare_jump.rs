use types::{MotionCommand, WorldState};

pub fn execute(_world_state: &WorldState) -> Option<MotionCommand> {
    Some(MotionCommand::ArmsUpSquat)
}
