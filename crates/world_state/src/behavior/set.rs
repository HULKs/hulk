use types::{
    motion_command::{HeadMotion, ImageRegion, MotionCommand},
    primary_state::PrimaryState,
    world_state::WorldState,
};

pub fn execute(world_state: &WorldState) -> Option<MotionCommand> {
    match world_state.robot.primary_state {
        PrimaryState::Set => Some(MotionCommand::Stand {
            head: (HeadMotion::Center {
                image_region_target: ImageRegion::Top,
            }),
        }),
        _ => None,
    }
}
