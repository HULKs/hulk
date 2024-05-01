use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct SolePressure {
    pub left: f32,
    pub right: f32,
}

impl SolePressure {
    pub fn total(&self) -> f32 {
        self.left + self.right
    }
}
