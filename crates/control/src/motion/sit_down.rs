use framework::{
    MainOutput, PersistentState, OptionalInput
};

pub struct SitDown {}

#[context]
pub struct NewContext {
    pub motion_safe_exits: PersistentState<MotionSafeExits, "motion_safe_exits">,
}

#[context]
pub struct CycleContext {


    pub motion_selection: OptionalInput<MotionSelection, "motion_selection">,
    pub sensor_data: OptionalInput<SensorData, "sensor_data">,



    pub motion_safe_exits: PersistentState<MotionSafeExits, "motion_safe_exits">,

}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub sit_down_joints_command: MainOutput<JointsCommand>,
}

impl SitDown {
    pub fn new(context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
