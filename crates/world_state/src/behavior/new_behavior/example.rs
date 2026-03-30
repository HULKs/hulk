use crate::{
    behavior::new_behavior::behavior_tree::{Node, Status, action, condition},
    selection, sequence,
};
use linear_algebra::vector;
use types::{
    motion_command::{HeadMotion, MotionCommand},
    primary_state::PrimaryState,
    world_state::{WorldState},
};

pub struct BehaviorContext {
    world_state: WorldState,
    some_other_data: i32,
}

pub fn execute(world_state: &WorldState) -> Option<MotionCommand> {
    let mut context = BehaviorContext {
        world_state: world_state.clone(),
        some_other_data: 0,
    };
    let behavior = ExampleBehavior::new(selection![
        sequence![condition(primary_state_is_safe), action(stand_still),],
        action(walk_to_ball),
    ]);
    behavior.cycle(&mut context)
}

pub struct ExampleBehavior {
    tree: Node<BehaviorContext, Option<MotionCommand>>,
}

impl ExampleBehavior {
    pub fn new(tree: Node<BehaviorContext, Option<MotionCommand>>) -> Self {
        Self { tree }
    }

    pub fn cycle(&self, context: &mut BehaviorContext) -> Option<MotionCommand> {
        match self.tree.tick(context) {
            Status::Success(command) | Status::Running(command) => command,
            _ => None,
        }
    }
}

// TEst funtions

fn primary_state_is_safe(context: &mut BehaviorContext) -> bool {
    context.world_state.robot.primary_state == PrimaryState::Safe
}

fn stand_still(context: &mut BehaviorContext) -> Status<Option<MotionCommand>> {
    //LOGIC
    context.some_other_data += 1;
    let command = MotionCommand::Stand {
        head: HeadMotion::LookAround,
    };
    Status::Success(Some(command))
}

fn walk_to_ball(_context: &mut BehaviorContext) -> Status<Option<MotionCommand>> {
    //LOGIC
    let command = MotionCommand::WalkWithVelocity {
        head: HeadMotion::LookAround,
        velocity: vector![2.0, 0.0],
        angular_velocity: 0.0,
    };
    Status::Success(Some(command))
}
