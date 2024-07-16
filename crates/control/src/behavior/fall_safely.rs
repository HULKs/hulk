use types::{fall_state::FallState, motion_command::MotionCommand, world_state::WorldState};

pub fn execute(world_state: &WorldState, has_firm_ground_contact: bool) -> Option<MotionCommand> {
    match (world_state.robot.fall_state, has_firm_ground_contact) {
        (FallState::Falling { direction, .. }, true) => {
            Some(MotionCommand::FallProtection { direction })
        }
        _ => None,
    }
}
