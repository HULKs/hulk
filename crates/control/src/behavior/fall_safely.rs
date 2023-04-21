use types::{FallState, MotionCommand, WorldState};

pub fn execute(world_state: &WorldState, has_ground_contact: &bool) -> Option<MotionCommand> {
    match (world_state.robot.fall_state, *has_ground_contact) {
        (FallState::Falling { direction }, true) => {
            Some(MotionCommand::FallProtection { direction })
        }
        _ => None,
    }
}
