use types::{camera_matrix::CameraMatrix, camera_position::CameraPosition};

use crate::lines::Lines;

#[derive(Clone)]
pub struct Measurement {
    pub position: CameraPosition,
    pub matrix: CameraMatrix,
    pub lines: Lines,
}
