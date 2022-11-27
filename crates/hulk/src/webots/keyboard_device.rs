use types::TouchSensors;
use webots::{Keyboard, Robot};

use super::interface::SIMULATION_TIME_STEP;

pub struct KeyboardDevice {
    keyboard: Keyboard,
}

impl Default for KeyboardDevice {
    fn default() -> Self {
        let keyboard = Robot::get_keyboard();
        keyboard.enable(SIMULATION_TIME_STEP);

        Self { keyboard }
    }
}

impl KeyboardDevice {
    pub fn get_touch_sensors(&self) -> TouchSensors {
        let key = self.keyboard.get_key();
        let control_shift_c_pressed = if let Some(key) = key {
            key == Keyboard::CONTROL | Keyboard::SHIFT | 'C' as u32
        } else {
            false
        };

        TouchSensors {
            chest_button: control_shift_c_pressed,
            head_front: false,
            head_middle: false,
            head_rear: false,
            left_foot_left: false,
            left_foot_right: false,
            left_hand_back: false,
            left_hand_left: false,
            left_hand_right: false,
            right_foot_left: false,
            right_foot_right: false,
            right_hand_back: false,
            right_hand_left: false,
            right_hand_right: false,
        }
    }
}
