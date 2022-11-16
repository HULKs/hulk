use context_attribute::context;
use framework::{MainOutput, Input, Parameter};
use types::{
    configuration::FallProtection, JointsCommand, MotionCommand, MotionSelection, SensorData,
};

pub struct FallProtector {}

#[context]
pub struct NewContext {
    pub fall_protection: Parameter<FallProtection, "control/fall_protection">,
}

#[context]
pub struct CycleContext {
    pub motion_command: Input<Option<MotionCommand>, "motion_command?">,
    pub motion_selection: Input<MotionSelection, "motion_selection?">,
    pub sensor_data: Input<SensorData, "sensor_data?">,

    pub fall_protection: Parameter<FallProtection, "control/fall_protection">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub fall_protection_command: MainOutput<JointsCommand>,
}

impl FallProtector {
    pub fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, _context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
