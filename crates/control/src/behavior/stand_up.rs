use types::{
    fall_state::{FallState, Variant},
    motion_command::{MotionCommand, StandUpVariant},
    world_state::WorldState,
};

pub fn execute(world_state: &WorldState) -> Option<MotionCommand> {
    match world_state.robot.fall_state {
        FallState::Fallen {
            variant: Variant::Front,
        } => Some(MotionCommand::StandUp {
            variant: StandUpVariant::Front,
        }),
        FallState::Fallen {
            variant: Variant::Back,
        } => Some(MotionCommand::StandUp {
            variant: StandUpVariant::Back,
        }),
        FallState::Fallen {
            variant: Variant::Sitting,
        } => Some(MotionCommand::StandUp {
            variant: StandUpVariant::Sitting,
        }),
        FallState::Fallen {
            variant: Variant::Squatting,
        } => Some(MotionCommand::StandUp {
            variant: StandUpVariant::Squatting,
        }),
        // If the robot is fallen with an unknown variant, we can't determine the best way to stand up. Trying back...
        FallState::Fallen {
            variant: Variant::Unknown,
        } => Some(MotionCommand::StandUp {
            variant: StandUpVariant::Back,
        }),
        _ => None,
    }
}
