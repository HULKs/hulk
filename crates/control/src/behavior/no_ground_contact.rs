use types::{
    motion_command::{HeadMotion, MotionCommand},
    world_state::WorldState,
};

pub fn execute(world_state: &WorldState) -> Option<MotionCommand> {
    if world_state.robot.has_ground_contact {
        return None;
    }
    Some(MotionCommand::Stand {
        head: HeadMotion::Center,
    })
}
