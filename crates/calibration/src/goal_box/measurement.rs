use serde::{Deserialize, Serialize};

use coordinate_systems::Pixel;
use projection::camera_matrix::CameraMatrix;

use super::lines::Lines;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Measurement {
    pub matrix: CameraMatrix,
    pub lines: Lines<Pixel>,
}
