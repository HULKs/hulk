use color_eyre::{Result, eyre::Error};

use context_attribute::context;
use framework::{AdditionalOutput, MainOutput};
use serde::{Deserialize, Serialize};
use types::{
    behavior_tree::{NodeTrace, Status},
    field_dimensions::FieldDimensions,
    motion_command::{HeadMotion, ImageRegion, MotionCommand},
    parameters::BehaviorParameters,
    path_obstacles::PathObstacle,
    world_state::WorldState,
};

use crate::behavior::{behavior_tree::Node, tree::create_tree};

fn create_tree_default() -> Node<Blackboard> {
    create_tree()
}

fn create_static_layout_default() -> NodeTrace {
    create_tree().static_layout_trace()
}

#[derive(Serialize, Deserialize)]
pub struct Behavior {
    #[serde(skip, default = "create_tree_default")]
    pub tree: Node<Blackboard>,
    #[serde(skip, default = "create_static_layout_default")]
    pub static_layout: NodeTrace,
    pub last_close_enough_to_kick: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Blackboard {
    pub world_state: WorldState,
    pub parameters: BehaviorParameters,
    pub field_dimensions: FieldDimensions,
    pub output: Option<MotionCommand>,
    pub last_close_enough_to_kick: bool,
    pub last_motion_command: MotionCommand,
    pub path_obstacles_output: Vec<PathObstacle>,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    world_state: Input<WorldState, "world_state">,

    field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    parameters: Parameter<BehaviorParameters, "behavior">,

    behavior_trace: AdditionalOutput<NodeTrace, "behavior.trace">,
    behavior_tree_layout: AdditionalOutput<NodeTrace, "behavior.tree_layout">,
    path_obstacles_output: AdditionalOutput<Vec<PathObstacle>, "path_obstacles">,

    last_motion_command: CyclerState<MotionCommand, "last_motion_command">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub motion_command: MainOutput<MotionCommand>,
}

impl Behavior {
    pub fn new(_context: CreationContext) -> Result<Self> {
        let tree = create_tree();
        let static_layout = tree.static_layout_trace();

        Ok(Self {
            tree,
            static_layout,
            last_close_enough_to_kick: false,
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        context
            .behavior_tree_layout
            .fill_if_subscribed(|| self.static_layout.clone());

        let mut blackboard = Blackboard {
            world_state: context.world_state.clone(),
            parameters: context.parameters.clone(),
            field_dimensions: *context.field_dimensions,
            output: None,
            last_close_enough_to_kick: self.last_close_enough_to_kick,
            last_motion_command: context.last_motion_command.clone(),
            path_obstacles_output: Vec::new(),
        };
        let (status, trace) = self.tree.tick_with_trace(&mut blackboard);
        context.behavior_trace.fill_if_subscribed(|| trace);
        context
            .path_obstacles_output
            .fill_if_subscribed(|| blackboard.path_obstacles_output);

        let motion_command: MotionCommand = match status {
            Status::Success => blackboard.output.take().unwrap_or(MotionCommand::Stand {
                head: HeadMotion::Center {
                    image_region_target: ImageRegion::Center,
                },
            }),
            Status::Failure => MotionCommand::Stand {
                head: HeadMotion::Center {
                    image_region_target: ImageRegion::Center,
                },
            },
            Status::Idle => {
                return Err(Error::msg(
                    "Behavior tree returned Idle status, which should not happen during a cycle",
                ));
            }
        };

        Ok(MainOutputs {
            motion_command: motion_command.into(),
        })
    }
}
