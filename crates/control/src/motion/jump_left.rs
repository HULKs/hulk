use context_attribute::context;
use framework::{MainOutput, PersistentState, RequiredInput};

pub struct JumpLeft {}

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
    pub jump_left_joints_command: MainOutput<JointsCommand>,
}

impl JumpLeft {
    pub fn new(context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
