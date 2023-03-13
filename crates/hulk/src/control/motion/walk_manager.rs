use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use types::{MotionCommand, MotionSelection, MotionType, Step, WalkCommand};

pub struct WalkManager {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    pub motion_command: Input<MotionCommand, "motion_command">,
    pub motion_selection: Input<MotionSelection, "motion_selection">,
    pub step_plan: Input<Step, "step_plan">,
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
                    kick, kicking_side, ..
                },
                MotionType::Walk,
            ) => WalkCommand::Kick(*kick, *kicking_side),
            _ => WalkCommand::Stand,
        };

        Ok(MainOutputs {
            walk_command: command.into(),
        })
    }
}
