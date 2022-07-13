use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

#[derive(Default, Clone, Serialize, Deserialize, SerializeHierarchy, Debug)]
pub struct Buttons {
    pub is_chest_button_pressed: bool,
    pub head_buttons_touched: bool,
    pub calibration_buttons_touched: bool,
}
