use types::{motion_command::MotionCommand, primary_state::PrimaryState, world_state::WorldState};

use crate::{
    behavior::behavior_tree::nodes::{Status, action, condition},
    selection, sequence,
};

pub fn execute(world_state: &WorldState) -> Option<MotionCommand> {
    let behavior_tree = selection![
        sequence![condition(primary_state_is_safe), action(stand_still),],
        action(walk_to_ball),
    ];
    None
}



// TEst funtions

fn primary_state_is_safe(world_state: &WorldState) -> Status {
    if world_state.robot.primary_state == PrimaryState::Safe {
        Status::Success
    } else {
        Status::Failure
    }
}

fn stand_still(_world_state: &WorldState) -> Status {
    //LOGIC
    Status::Success
}

fn walk_to_ball(_world_state: &WorldState) -> Status {
    //LOGIC
    Status::Success
}
