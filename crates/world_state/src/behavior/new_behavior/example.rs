use crate::{
    behavior::new_behavior::behavior_tree::{Status},
    selection, sequence, condition, action,
};
use linear_algebra::vector;
use types::{
    motion_command::{HeadMotion, MotionCommand},
    primary_state::PrimaryState,
    world_state::WorldState,
};

pub struct BehaviorContext {
    world_state: WorldState,
    some_other_data: i32,

    motion_command: Option<MotionCommand>,
}

pub fn execute(world_state: &WorldState) -> Option<MotionCommand> {
    let mut context = BehaviorContext {
        world_state: world_state.clone(),
        some_other_data: 0,
        motion_command: None,
    };
    let tree = selection![
        sequence![condition!(primary_state_is_safe), action!(stand_still),],
        action!(walk_to_ball),
    ];
    tree.tick(&mut context);

    context.motion_command
}


// TEst funtions

fn primary_state_is_safe(context: &mut BehaviorContext) -> bool {
    context.world_state.robot.primary_state == PrimaryState::Safe
}

fn stand_still(context: &mut BehaviorContext) -> Status {
    //LOGIC
    context.some_other_data += 1;
    context.motion_command = Some(MotionCommand::Stand {
        head: HeadMotion::LookAround,
    });
    Status::Success
}

fn walk_to_ball(context: &mut BehaviorContext) -> Status {
    //LOGIC
    context.motion_command = Some(MotionCommand::WalkWithVelocity {
        head: HeadMotion::LookAround,
        velocity: vector![2.0, 0.0],
        angular_velocity: 0.0,
    });
    Status::Success
}
