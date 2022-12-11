use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use types::{
    configuration::FallProtection, JointsCommand, MotionCommand, MotionSelection, SensorData,
};

pub struct FallProtector {}

#[context]
pub struct CreationContext {
    pub fall_protection: Parameter<FallProtection, "control.fall_protection">,
}

#[context]
pub struct CycleContext {
    pub motion_command: RequiredInput<Option<MotionCommand>, "motion_command?">,
    pub motion_selection: RequiredInput<Option<MotionSelection>, "motion_selection?">,
    pub sensor_data: Input<SensorData, "sensor_data">,

    pub fall_protection: Parameter<FallProtection, "control.fall_protection">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub fall_protection_command: MainOutput<Option<JointsCommand>>,
}

impl FallProtector {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, _context: CycleContext) -> Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
