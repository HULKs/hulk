use context_attribute::context;
use framework::{MainOutput, OptionalInput, Parameter};

pub struct FallProtector {}

#[context]
pub struct NewContext {
    pub fall_protection: Parameter<FallProtection, "control/fall_protection">,
}

#[context]
pub struct CycleContext {
    pub motion_command: OptionalInput<MotionCommand, "motion_command">,
    pub motion_selection: OptionalInput<MotionSelection, "motion_selection">,
    pub sensor_data: OptionalInput<SensorData, "sensor_data">,

    pub fall_protection: Parameter<FallProtection, "control/fall_protection">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub fall_protection_command: MainOutput<JointsCommand>,
}

impl FallProtector {
    pub fn new(context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
