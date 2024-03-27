use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

#[derive(
    Default, Clone, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect, Debug,
)]
pub struct Buttons {
    pub is_chest_button_pressed_once: bool,
    pub is_chest_button_pressed_twice: bool,
    pub head_buttons_touched: bool,
    pub calibration_buttons_touched: bool,
}
