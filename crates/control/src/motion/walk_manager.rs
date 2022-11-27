use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use types::{MotionCommand, MotionSelection, Step, WalkCommand};

pub struct WalkManager {}

#[context]
pub struct NewContext {}

#[context]
pub struct CycleContext {
    pub motion_command: RequiredInput<Option<MotionCommand>, "motion_command?">,
    pub motion_selection: RequiredInput<Option<MotionSelection>, "motion_selection?">,
    pub step_plan: RequiredInput<Option<Step>, "step_plan?">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub walk_command: MainOutput<Option<WalkCommand>>,
}

impl WalkManager {
    pub fn new(_context: NewContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, _context: CycleContext) -> Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
