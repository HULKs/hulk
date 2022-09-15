use framework::{
    MainOutput, PersistentState, OptionalInput
};

pub struct MotionSelector {}

#[context]
pub struct NewContext {
    pub motion_safe_exits: PersistentState<MotionSafeExits, "motion_safe_exits">,
}

#[context]
pub struct CycleContext {


    pub motion_command: OptionalInput<MotionCommand, "motion_command">,



    pub motion_safe_exits: PersistentState<MotionSafeExits, "motion_safe_exits">,

}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub motion_selection: MainOutput<MotionSelection>,
}

impl MotionSelector {
    pub fn new(context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
