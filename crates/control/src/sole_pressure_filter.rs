use context_attribute::context;
use framework::{MainOutput, Input};
use types::{SensorData, SolePressure};

pub struct SolePressureFilter {}

#[context]
pub struct NewContext {}

#[context]
pub struct CycleContext {
    pub sensor_data: Input<SensorData, "sensor_data?">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub sole_pressure: MainOutput<SolePressure>,
}

impl SolePressureFilter {
    pub fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, _context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
