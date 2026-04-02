use color_eyre::{Result, eyre::Error};

use context_attribute::context;
use framework::{AdditionalOutput, MainOutput};
use serde::{Deserialize, Serialize};
use types::{
    behavior_tree::{NodeTrace, Status},
    motion_command::{HeadMotion, ImageRegion, MotionCommand},
    parameters::BehaviorParameters,
    world_state::WorldState,
};

use crate::behavior::{behavior_tree::Node, tree::create_tree};

fn create_tree_default() -> Node<CaptainBlackboard> {
    create_tree()
}

fn create_static_layout_default() -> NodeTrace {
    create_tree().static_layout_trace()
}

#[derive(Serialize, Deserialize)]
pub struct Behavior {
    #[serde(skip_deserializing, default = "create_tree_default")]
    pub tree: Node<CaptainBlackboard>,
    #[serde(skip_deserializing, default = "create_static_layout_default")]
    pub static_layout: NodeTrace,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptainBlackboard {
    pub world_state: WorldState,
    pub parameters: BehaviorParameters,
    pub output: Option<MotionCommand>,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    world_state: Input<WorldState, "world_state">,
    parameters: Parameter<BehaviorParameters, "behavior">,
    behavior_trace: AdditionalOutput<NodeTrace, "behavior.trace">,
    behavior_tree_layout: AdditionalOutput<NodeTrace, "behavior.tree_layout">,
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
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        context
            .behavior_tree_layout
            .fill_if_subscribed(|| self.static_layout.clone());

        let mut blackboard = CaptainBlackboard {
            world_state: context.world_state.clone(),
            parameters: context.parameters.clone(),
            output: None,
        };

        let (status, trace) = self.tree.tick_with_trace(&mut blackboard);
        context.behavior_trace.fill_if_subscribed(|| trace);

        let motion_command: MotionCommand = match status {
            Status::Success | Status::Running => {
                blackboard.output.take().unwrap_or(MotionCommand::Stand {
                    head: HeadMotion::Center {
                        image_region_target: ImageRegion::Center,
                    },
                })
            }
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
