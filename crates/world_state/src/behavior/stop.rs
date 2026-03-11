use types::{
    motion_command::{HeadMotion, MotionCommand},
    primary_state::PrimaryState,
    world_state::WorldState,
};

pub fn execute(world_state: &WorldState) -> Option<MotionCommand> {
    match world_state.robot.primary_state {
        PrimaryState::Stop => Some(MotionCommand::Stand {
            head: (HeadMotion::ZeroAngles),
        }),
        _ => None,
    }
}
