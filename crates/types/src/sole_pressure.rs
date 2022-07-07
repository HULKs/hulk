use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct SolePressure {
    pub left: f32,
    pub right: f32,
}

impl SolePressure {
    pub fn total(&self) -> f32 {
        self.left + self.right
    }
}
