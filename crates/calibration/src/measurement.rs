use coordinate_systems::Pixel;
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;
use types::{camera_matrix::CameraMatrix, camera_position::CameraPosition};

use crate::lines::Lines;

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct Measurement {
    pub position: CameraPosition,
    pub matrix: CameraMatrix,
    pub lines: Lines<Pixel>,
}
