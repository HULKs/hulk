use context_attribute::context;
use framework::{MainOutput, Parameter, RequiredInput};

pub struct SupportFootEstimation {}

#[context]
pub struct NewContext {
    pub hysteresis: Parameter<f32, "control/support_foot_estimation/hysteresis">,
}

#[context]
pub struct CycleContext {
    pub hysteresis: Parameter<f32, "control/support_foot_estimation/hysteresis">,

    pub has_ground_contact: RequiredInput<bool, "has_ground_contact">,
    pub sensor_data: RequiredInput<SensorData, "sensor_data">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub support_foot: MainOutput<SupportFoot>,
}

impl SupportFootEstimation {
    pub fn new(context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
