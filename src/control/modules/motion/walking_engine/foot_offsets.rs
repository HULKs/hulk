use macros::SerializeHierarchy;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, SerializeHierarchy)]
pub struct FootOffsets {
    pub forward: f32,
    pub left: f32,
}

impl FootOffsets {
    pub fn zero() -> Self {
        Self {
            forward: 0.0,
            left: 0.0,
        }
    }
}
