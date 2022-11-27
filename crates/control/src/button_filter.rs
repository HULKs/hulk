use std::time::Duration;

use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use types::{Buttons, SensorData};

pub struct ButtonFilter {}

#[context]
pub struct NewContext {
    pub calibration_buttons_timeout:
        Parameter<Duration, "control/button_filter/calibration_buttons_timeout">,
    pub head_buttons_timeout: Parameter<Duration, "control/button_filter/head_buttons_timeout">,
}

#[context]
pub struct CycleContext {
    pub sensor_data: Input<SensorData, "sensor_data">,

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
    pub fn new(_context: NewContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, _context: CycleContext) -> Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
