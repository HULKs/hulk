use types::{fall_state::FallState, motion_command::MotionCommand, world_state::WorldState};

pub fn execute(world_state: &WorldState) -> Option<MotionCommand> {
    match world_state.robot.fall_state {
        FallState::Fallen { facing } => Some(MotionCommand::StandUp { facing }),
        _ => None,
    }
}
