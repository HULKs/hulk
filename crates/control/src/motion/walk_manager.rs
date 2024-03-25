use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use serde::{Deserialize, Serialize};
use types::{
    motion_command::MotionCommand,
    motion_selection::{MotionSelection, MotionType},
    step_plan::Step,
    walk_command::WalkCommand,
};

#[derive(Deserialize, Serialize)]
pub struct WalkManager {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    motion_command: Input<MotionCommand, "motion_command">,
    motion_selection: Input<MotionSelection, "motion_selection">,
    step_plan: Input<Step, "step_plan">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub walk_command: MainOutput<WalkCommand>,
}

impl WalkManager {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        if !(matches!(context.motion_selection.current_motion, MotionType::Walk)) {
            return Ok(MainOutputs {
                walk_command: WalkCommand::Stand.into(),
            });
        }

        let command = match context.motion_command {
            MotionCommand::Walk { .. } => WalkCommand::Walk {
                step: *context.step_plan,
            },
            MotionCommand::InWalkKick {
                kick,
                kicking_side,
                strength,
                ..
            } => WalkCommand::Kick {
                variant: *kick,
                side: *kicking_side,
                strength: *strength,
            },
            _ => WalkCommand::Stand,
        };

        Ok(MainOutputs {
            walk_command: command.into(),
        })
    }
}
