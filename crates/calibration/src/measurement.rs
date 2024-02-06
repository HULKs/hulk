use types::{
    camera_matrix::CameraMatrix, camera_position::CameraPosition, coordinate_systems::Pixel,
};

use crate::lines::Lines;

#[derive(Clone)]
pub struct Measurement {
    pub position: CameraPosition,
    pub matrix: CameraMatrix,
    pub lines: Lines<Pixel>,
}
