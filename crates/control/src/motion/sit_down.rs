use context_attribute::context;
use framework::{MainOutput, OptionalInput, PersistentState};
use types::{JointsCommand, MotionSafeExits, MotionSelection, SensorData};

pub struct SitDown {}

#[context]
pub struct NewContext {
    pub motion_safe_exits: PersistentState<MotionSafeExits, "motion_safe_exits">,
}

#[context]
pub struct CycleContext {
    pub motion_selection: OptionalInput<MotionSelection, "motion_selection?">,
    pub sensor_data: OptionalInput<SensorData, "sensor_data?">,

    pub motion_safe_exits: PersistentState<MotionSafeExits, "motion_safe_exits">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub sit_down_joints_command: MainOutput<JointsCommand>,
}

impl SitDown {
    pub fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, _context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
