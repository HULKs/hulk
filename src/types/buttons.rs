use macros::SerializeHierarchy;
use serde::{Deserialize, Serialize};

#[derive(Default, Clone, Serialize, Deserialize, SerializeHierarchy, Debug)]
pub struct Buttons {
    pub is_chest_button_pressed: bool,
    pub are_all_head_elements_touched: bool,
}
