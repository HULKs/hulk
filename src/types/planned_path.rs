use macros::SerializeHierarchy;
use nalgebra::Isometry2;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Serialize, SerializeHierarchy, Deserialize)]
pub struct PlannedPath {
    pub end_pose: Isometry2<f32>,
}
