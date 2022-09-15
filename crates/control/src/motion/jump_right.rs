use context_attribute::context;
use framework::{MainOutput, PersistentState, RequiredInput};

pub struct JumpRight {}

#[context]
pub struct NewContext {
    pub motion_safe_exits: PersistentState<MotionSafeExits, "motion_safe_exits">,
}

#[context]
pub struct CycleContext {
    pub motion_safe_exits: PersistentState<MotionSafeExits, "motion_safe_exits">,

    pub motion_selection: RequiredInput<MotionSelection, "motion_selection">,
    pub sensor_data: RequiredInput<SensorData, "sensor_data">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub jump_right_joints_command: MainOutput<JointsCommand>,
}

impl JumpRight {
    pub fn new(context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
