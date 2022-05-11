use macros::SerializeHierarchy;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct SolePressure {
    pub left: f32,
    pub right: f32,
}
