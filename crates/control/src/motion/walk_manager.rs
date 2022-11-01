use context_attribute::context;
use framework::{MainOutput, OptionalInput};
use types::{MotionCommand, MotionSelection, Step, WalkCommand};

pub struct WalkManager {}

#[context]
pub struct NewContext {}

#[context]
pub struct CycleContext {
    pub motion_command: OptionalInput<MotionCommand, "motion_command?">,
    pub motion_selection: OptionalInput<MotionSelection, "motion_selection?">,
    pub step_plan: OptionalInput<Step, "step_plan?">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub walk_command: MainOutput<WalkCommand>,
}

impl WalkManager {
    pub fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, _context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
