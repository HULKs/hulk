use context_attribute::context;
use framework::MainOutput;
use types::{MotionCommand, MotionSafeExits, MotionSelection};

pub struct MotionSelector {}

#[context]
pub struct NewContext {
    pub motion_safe_exits: PersistentState<MotionSafeExits, "motion_safe_exits">,
}

#[context]
pub struct CycleContext {
    pub motion_command: RequiredInput<Option<MotionCommand>, "motion_command?">,

    pub motion_safe_exits: PersistentState<MotionSafeExits, "motion_safe_exits">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub motion_selection: MainOutput<Option<MotionSelection>>,
}

impl MotionSelector {
    pub fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, _context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
