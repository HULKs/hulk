use std::time::{Duration, SystemTime};

use filtering::TapDetector;
use types::{Buttons, SensorData};

#[derive(Default)]
pub struct ButtonFilter {
    chest_button_tap_detector: TapDetector,
    head_buttons_touched: SystemTime,
    last_head_buttons_touched: bool,
    calibration_buttons_touched: SystemTime,
    last_calibration_buttons_touched: bool,
}

#[context]
pub struct NewContext {}

#[context]
pub struct CycleContext {
    pub head_buttons_timeout: Parameter<Duration, "control/button_filter/head_buttons_timeout">,
    pub calibration_buttons_timeout:
        Parameter<Duration, "control/button_filter/calibration_buttons_timeout">,

    pub sensor_data: RequiredInput<SensorData, "sensor_data">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub buttons: MainOutput<Buttons>,
}

impl ButtonFilter {
    pub fn new(context: NewContext) -> anyhow::Result<Self> {
        Ok(Self::default()) // TODO: This is wrong
    }

    pub fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
