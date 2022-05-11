use macros::{module, require_some};

use crate::{
    control::filtering::TapDetector,
    types::{Buttons, SensorData},
};

pub struct ButtonFilter {
    chest_button_tap_detector: TapDetector,
}

#[module(control)]
#[input(path = sensor_data, data_type = SensorData)]
#[main_output(data_type = Buttons)]
impl ButtonFilter {}

impl ButtonFilter {
    pub fn new() -> Self {
        Self {
            chest_button_tap_detector: TapDetector::new(),
        }
    }

    fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        let touch_sensors = &require_some!(context.sensor_data).touch_sensors;
        self.chest_button_tap_detector
            .update(touch_sensors.chest_button);
        let are_all_head_elements_touched =
            touch_sensors.head_front && touch_sensors.head_middle && touch_sensors.head_rear;
        Ok(MainOutputs {
            buttons: Some(Buttons {
                is_chest_button_pressed: self.chest_button_tap_detector.is_single_tapped(),
                are_all_head_elements_touched,
            }),
        })
    }
}
