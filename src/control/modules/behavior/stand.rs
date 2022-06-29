use crate::types::{MotionCommand, PrimaryState, WorldState};

use super::head::look_for_ball;

pub fn execute(world_state: &WorldState) -> Option<MotionCommand> {
    match (
        world_state.robot.primary_state,
        world_state.robot.robot_to_field,
    ) {
        (PrimaryState::Initial | PrimaryState::Set, _) | (_, None) => Some(MotionCommand::Stand {
            head: look_for_ball(world_state.ball),
        }),
        _ => None,
    }
}
