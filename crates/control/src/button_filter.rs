use std::time::Duration;

use color_eyre::Result;
use context_attribute::context;
use filtering::{debounce_button::DebounceButton, tap_detector::TapDetector};
use framework::MainOutput;
use serde::{Deserialize, Serialize};
use types::{buttons::Buttons, cycle_time::CycleTime, sensor_data::SensorData};

#[derive(Deserialize, Serialize)]
pub struct ButtonFilter {
    chest_button_tap_detector: TapDetector,
    debounced_head_button: DebounceButton,
    debounced_calibration_button: DebounceButton,
    debounced_animation_button: DebounceButton,
    animation_button_released: TapDetector,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    sensor_data: Input<SensorData, "sensor_data">,
    cycle_time: Input<CycleTime, "cycle_time">,

    calibration_buttons_timeout: Parameter<Duration, "button_filter.calibration_buttons_timeout">,
    head_buttons_timeout: Parameter<Duration, "button_filter.head_buttons_timeout">,
    animation_button_timeout: Parameter<Duration, "button_filter.animation_button_timeout">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub buttons: MainOutput<Buttons>,
}

impl ButtonFilter {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            chest_button_tap_detector: TapDetector::default(),
            debounced_head_button: DebounceButton::default(),
            debounced_calibration_button: DebounceButton::default(),
            debounced_animation_button: DebounceButton::default(),
            animation_button_released: TapDetector::default(),
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let head_buttons_timeout = *context.head_buttons_timeout;
        let calibration_buttons_timeout = *context.calibration_buttons_timeout;
        let touch_sensors = &context.sensor_data.touch_sensors;
        let animation_button_timeout = *context.animation_button_timeout;

        self.chest_button_tap_detector
            .update(touch_sensors.chest_button);

        let head_buttons_touched =
            touch_sensors.head_front && touch_sensors.head_middle && touch_sensors.head_rear;
        let debounced_head_buttons_touched = self.debounced_head_button.debounce_button(
            head_buttons_touched,
            context.cycle_time.start_time,
            head_buttons_timeout,
        );

        let calibration_buttons_touched = touch_sensors.chest_button && touch_sensors.head_front;
        let debounced_calibration_buttons_touched =
            self.debounced_calibration_button.debounce_button(
                calibration_buttons_touched,
                context.cycle_time.start_time,
                calibration_buttons_timeout,
            );

        let animation_buttons_touched = touch_sensors.head_rear;

        let debounced_animation_buttons_touched = self.debounced_animation_button.debounce_button(
            animation_buttons_touched,
            context.cycle_time.start_time,
            animation_button_timeout,
        );

        Ok(MainOutputs {
            buttons: Buttons {
                is_chest_button_pressed_once: self.chest_button_tap_detector.is_single_tapped(),
                head_buttons_touched: debounced_head_buttons_touched,
                calibration_buttons_touched: debounced_calibration_buttons_touched,
                animation_buttons_touched: debounced_animation_buttons_touched,
            }
            .into(),
        })
    }
}
