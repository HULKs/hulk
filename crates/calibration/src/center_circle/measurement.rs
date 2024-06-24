use super::circles::CenterOfCircleAndPoints;
use coordinate_systems::Pixel;
use projection::camera_matrix::CameraMatrix;
use types::camera_position::CameraPosition;

#[derive(Clone)]
pub struct Measurement {
    pub position: CameraPosition,
    pub matrix: CameraMatrix,
    pub circles: CenterOfCircleAndPoints<Pixel>,
}
