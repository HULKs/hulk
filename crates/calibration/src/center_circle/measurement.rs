use serde::{Deserialize, Serialize};

use coordinate_systems::Pixel;
use projection::camera_matrix::CameraMatrix;

use super::circle_points::CenterCirclePoints;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Measurement {
    pub matrix: CameraMatrix,
    pub circle_and_points: CenterCirclePoints<Pixel>,
}
