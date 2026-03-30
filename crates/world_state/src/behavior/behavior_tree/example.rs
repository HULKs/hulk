use linear_algebra::vector;
use types::{
    motion_command::{HeadMotion, MotionCommand},
    primary_state::PrimaryState,
    world_state::WorldState,
};
use crate::{
    behavior::behavior_tree::nodes::{Node, Status, action, condition},
    selection, sequence,
};



pub fn execute(world_state: &WorldState) -> Option<MotionCommand> {
    let behavior = ExampleBehavior::new(selection![
        sequence![condition(primary_state_is_safe), action(stand_still),],
        action(walk_to_ball),
    ]);
    behavior.cycle(world_state)
}

pub struct ExampleBehavior {
    tree: Node<WorldState, Option<MotionCommand>>,
}

impl ExampleBehavior {
    pub fn new(tree: Node<WorldState, Option<MotionCommand>>) -> Self {
        Self { tree }
    }

    pub fn cycle(&self, world_state: &WorldState) -> Option<MotionCommand> {
        match self.tree.tick(world_state) {
            Status::Success(command) | Status::Running(command) => command,
            _ => None,
        }
    }
}

// TEst funtions

fn primary_state_is_safe(world_state: &WorldState) -> bool {
    world_state.robot.primary_state == PrimaryState::Safe
}

fn stand_still(_world_state: &WorldState) -> Status<Option<MotionCommand>> {
    //LOGICStatus<Option<MotionCommand>
    let command = MotionCommand::Stand {
        head: HeadMotion::LookAround,
    };
    Status::Success(Some(command))
}

fn walk_to_ball(_world_state: &WorldState) -> Status<Option<MotionCommand>> {
    //LOGIC
    let command = MotionCommand::WalkWithVelocity {
        head: HeadMotion::LookAround,
        velocity: vector![2.0, 0.0],
        angular_velocity: 0.0,
    };
    Status::Success(Some(command))
}
