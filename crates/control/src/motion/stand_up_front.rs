use context_attribute::context;
use framework::{MainOutput, OptionalInput, Parameter, PersistentState};

pub struct StandUpFront {}

#[context]
pub struct NewContext {
    pub gyro_low_pass_filter_coefficient:
        Parameter<f32, "control/stand_up/gyro_low_pass_filter_coefficient">,
    pub gyro_low_pass_filter_tolerance:
        Parameter<f32, "control/stand_up/gyro_low_pass_filter_tolerance">,

    pub motion_safe_exits: PersistentState<MotionSafeExits, "motion_safe_exits">,
}

#[context]
pub struct CycleContext {
    pub motion_command: OptionalInput<MotionCommand, "motion_command">,
    pub motion_selection: OptionalInput<MotionSelection, "motion_selection">,
    pub sensor_data: OptionalInput<SensorData, "sensor_data">,

    pub gyro_low_pass_filter_coefficient:
        Parameter<f32, "control/stand_up/gyro_low_pass_filter_coefficient">,
    pub gyro_low_pass_filter_tolerance:
        Parameter<f32, "control/stand_up/gyro_low_pass_filter_tolerance">,

    pub motion_safe_exits: PersistentState<MotionSafeExits, "motion_safe_exits">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub stand_up_front_positions: MainOutput<Joints>,
}

impl StandUpFront {
    pub fn new(context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
