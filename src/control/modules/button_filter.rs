use std::time::{Duration, SystemTime, UNIX_EPOCH};

use module_derive::{module, require_some};
use types::{Buttons, SensorData};

use crate::control::filtering::TapDetector;

pub struct ButtonFilter {
    chest_button_tap_detector: TapDetector,
    head_buttons_touched: SystemTime,
    last_head_buttons_touched: bool,
}

#[module(control)]
#[input(path = sensor_data, data_type = SensorData)]
#[parameter(path = control.button_filter.head_buttons_timeout, data_type = Duration)]
#[main_output(data_type = Buttons)]
impl ButtonFilter {}

impl ButtonFilter {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {
            chest_button_tap_detector: TapDetector::new(),
            head_buttons_touched: UNIX_EPOCH,
            last_head_buttons_touched: false,
        })
    }

    fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        let touch_sensors = &require_some!(context.sensor_data).touch_sensors;
        let cycle_start_time = require_some!(context.sensor_data).cycle_info.start_time;
        let timeout = *context.head_buttons_timeout;

        self.chest_button_tap_detector
            .update(touch_sensors.chest_button);

        let head_buttons_touched =
            touch_sensors.head_front && touch_sensors.head_middle && touch_sensors.head_rear;

        let head_buttons_touched_initially =
            head_buttons_touched && !self.last_head_buttons_touched;
        if head_buttons_touched_initially {
            self.head_buttons_touched = cycle_start_time;
        }
        self.last_head_buttons_touched = head_buttons_touched;

        let debounced_head_buttons_touched = head_buttons_touched
            && cycle_start_time
                .duration_since(self.head_buttons_touched)
                .unwrap()
                >= timeout;

        Ok(MainOutputs {
            buttons: Some(Buttons {
                is_chest_button_pressed: self.chest_button_tap_detector.is_single_tapped(),
                head_buttons_touched: debounced_head_buttons_touched,
            }),
        })
    }
}
