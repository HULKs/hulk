use types::{fall_state::FallState, motion_command::MotionCommand, world_state::WorldState};

pub fn execute(world_state: &WorldState) -> Option<MotionCommand> {
    match (
        world_state.robot.fall_state,
        world_state.robot.stand_up_count,
    ) {
        (FallState::Fallen { kind }, 0) => Some(MotionCommand::StandUp { kind }),
        (FallState::StandingUp { kind, .. }, 0) => Some(MotionCommand::StandUp { kind }),
        (FallState::Fallen { .. }, 1) => Some(MotionCommand::Penalized),
        (FallState::StandingUp { .. }, 1) => Some(MotionCommand::Penalized),
        _ => None,
    }
}
