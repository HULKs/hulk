use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use types::{SensorData, SupportFoot};

pub struct SupportFootEstimation {}

#[context]
pub struct NewContext {
    pub hysteresis: Parameter<f32, "control/support_foot_estimation/hysteresis">,
}

#[context]
pub struct CycleContext {
    pub hysteresis: Parameter<f32, "control/support_foot_estimation/hysteresis">,

    pub has_ground_contact: Input<bool, "has_ground_contact">,
    pub sensor_data: Input<SensorData, "sensor_data">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub support_foot: MainOutput<Option<SupportFoot>>,
}

impl SupportFootEstimation {
    pub fn new(_context: NewContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, _context: CycleContext) -> Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
