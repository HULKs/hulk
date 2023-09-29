use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use types::{
    motion_command::MotionCommand,
    motion_selection::{MotionSelection, MotionType},
    step_plan::Step,
    walk_command::WalkCommand,
};

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
        let command = match (
            context.motion_command,
            context.motion_selection.current_motion,
        ) {
            (MotionCommand::Walk { .. }, MotionType::Walk) => WalkCommand::Walk(*context.step_plan),
            (
                MotionCommand::InWalkKick {
                    kick,
                    kicking_side,
                    strength,
                    ..
                },
                MotionType::Walk,
            ) => WalkCommand::Kick(*kick, *kicking_side, *strength),
            _ => WalkCommand::Stand,
        };

        Ok(MainOutputs {
            walk_command: command.into(),
        })
    }
}
