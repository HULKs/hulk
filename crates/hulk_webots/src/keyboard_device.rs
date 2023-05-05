use types::TouchSensors;
use webots::{Keyboard, Receiver, Robot};

use super::hardware_interface::SIMULATION_TIME_STEP;

pub struct KeyboardDevice {
    keyboard: Keyboard,
    receiver: Receiver,
}

impl Default for KeyboardDevice {
    fn default() -> Self {
        let keyboard = Robot::get_keyboard();
        keyboard.enable(SIMULATION_TIME_STEP);
        let receiver = Robot::get_receiver("ChestButton Channel");
        receiver.enable(SIMULATION_TIME_STEP);
        Self { keyboard, receiver }
    }
}

impl KeyboardDevice {
    pub fn get_touch_sensors(&self) -> TouchSensors {
        let received_message = match self.receiver.get_next_packet() {
            Ok(message) => message,
            Err(error) => {
                println!("error in received message: {error:?}");
                None
            }
        };

        let (control_shift_c_pressed, control_shift_x_pressed, control_shift_u_pressed) =
            if let Some(key) = self.keyboard.get_key() {
                const CONTROL_SHIFT_MASK: u32 = Keyboard::CONTROL | Keyboard::SHIFT;
                (
                    key == CONTROL_SHIFT_MASK | 'C' as u32,
                    key == CONTROL_SHIFT_MASK | 'X' as u32,
                    key == CONTROL_SHIFT_MASK | 'U' as u32,
                )
            } else {
                (false, false, false)
            };

        TouchSensors {
            chest_button: received_message.is_some()
                || control_shift_c_pressed
                || control_shift_x_pressed,
            head_front: control_shift_u_pressed || control_shift_x_pressed,
            head_middle: control_shift_u_pressed,
            head_rear: control_shift_u_pressed,
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
