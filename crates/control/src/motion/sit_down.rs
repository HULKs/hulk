use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use types::{JointsCommand, MotionSafeExits, MotionSelection, SensorData};

pub struct SitDown {}

#[context]
pub struct NewContext {
    pub motion_safe_exits: PersistentState<MotionSafeExits, "motion_safe_exits">,
}

#[context]
pub struct CycleContext {
    pub motion_selection: RequiredInput<Option<MotionSelection>, "motion_selection?">,
    pub sensor_data: Input<SensorData, "sensor_data">,

    pub motion_safe_exits: PersistentState<MotionSafeExits, "motion_safe_exits">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub sit_down_joints_command: MainOutput<Option<JointsCommand>>,
}

impl SitDown {
    pub fn new(_context: NewContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, _context: CycleContext) -> Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
