use types::{motion_command::MotionCommand, world_state::WorldState};

pub fn execute(_world_state: &WorldState) -> Option<MotionCommand> {
    Some(MotionCommand::ArmsUpSquat)
}
