use context_attribute::context;
use framework::{MainOutput, OptionalInput, Parameter};

pub struct ButtonFilter {}

#[context]
pub struct NewContext {
    pub calibration_buttons_timeout:
        Parameter<Duration, "control/button_filter/calibration_buttons_timeout">,
    pub head_buttons_timeout: Parameter<Duration, "control/button_filter/head_buttons_timeout">,
}

#[context]
pub struct CycleContext {
    pub sensor_data: OptionalInput<SensorData, "sensor_data?">,

    pub calibration_buttons_timeout:
        Parameter<Duration, "control/button_filter/calibration_buttons_timeout">,
    pub head_buttons_timeout: Parameter<Duration, "control/button_filter/head_buttons_timeout">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub buttons: MainOutput<Buttons>,
}

impl ButtonFilter {
    pub fn new(context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
