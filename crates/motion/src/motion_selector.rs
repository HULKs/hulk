use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use serde::{Deserialize, Serialize};
use types::motion_command::MotionCommand;

#[derive(Deserialize, Serialize)]
pub struct MotionSelector {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    motion_command_from_behavior: Input<MotionCommand, "WorldState", "motion_command">,
    motion_command_from_remote_control: Input<MotionCommand, "motion_command">,
    use_remote_control_for_motion_selection:
        Parameter<bool, "motion.use_remote_control_for_motion_selection">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub selected_motion_command: MainOutput<MotionCommand>,
}

impl MotionSelector {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let seleciton: MotionCommand = if *context.use_remote_control_for_motion_selection {
            context.motion_command_from_remote_control.clone()
        } else {
            context.motion_command_from_behavior.clone()
        };
        Ok(MainOutputs {
            selected_motion_command: seleciton.into(),
        })
    }
}
