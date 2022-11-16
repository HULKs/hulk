use context_attribute::context;
use framework::{Input, MainOutput, PersistentState, RequiredInput};
use types::{JointsCommand, MotionSafeExits, MotionSelection, SensorData};

pub struct ArmsUpSquat {}

#[context]
pub struct NewContext {
    pub motion_safe_exits: PersistentState<MotionSafeExits, "motion_safe_exits">,
}

#[context]
pub struct CycleContext {
    pub motion_safe_exits: PersistentState<MotionSafeExits, "motion_safe_exits">,

    pub motion_selection: RequiredInput<Option<MotionSelection>, "motion_selection?">,
    pub sensor_data: Input<SensorData, "sensor_data">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub arms_up_squat_joints_command: MainOutput<Option<JointsCommand>>,
}

impl ArmsUpSquat {
    pub fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, _context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
