use macros::SerializeHierarchy;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct GroundContact {
    pub left_foot: bool,
    pub right_foot: bool,
}

impl GroundContact {
    pub fn any_foot(&self) -> bool {
        self.left_foot || self.right_foot
    }
}
