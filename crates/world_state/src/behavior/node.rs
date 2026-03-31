use color_eyre::Result;

use context_attribute::context;
use framework::{AdditionalOutput, MainOutput};
use serde::{Deserialize, Serialize};
use types::{
    behavior_tree::{NodeTrace, Status}, motion_command::{HeadMotion, ImageRegion, MotionCommand}, parameters::BehaviorParameters, world_state::WorldState
};

use crate::behavior::{behavior_tree::Node, tree};



#[derive(Serialize)]
pub struct Behavior {
    pub tree: Node<CaptainBlackboard>,
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
    behavior_trace: AdditionalOutput<NodeTrace, "behavior_trace">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub motion_command: MainOutput<MotionCommand>,
}

impl Behavior {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            tree: tree::create_tree(),
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
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
        };

        Ok(MainOutputs {
            motion_command: motion_command.into(),
        })
    }
}
