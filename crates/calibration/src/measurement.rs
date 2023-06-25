use types::{CameraMatrix, CameraPosition};

use crate::lines::Lines;

#[derive(Clone, Debug)]
pub struct Measurement {
    pub position: CameraPosition,
    pub matrix: CameraMatrix,
    pub lines: Lines,
}
