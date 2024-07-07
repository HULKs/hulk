use serde::{Deserialize, Serialize};

use coordinate_systems::Pixel;
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use projection::camera_matrix::CameraMatrix;
use types::camera_position::CameraPosition;

use super::circle_points::CenterCirclePoints;

#[derive(
    Clone, Debug, Default, Serialize, Deserialize, PathSerialize, PathIntrospect, PathDeserialize,
)]
pub struct Measurement {
    pub position: CameraPosition,
    pub matrix: CameraMatrix,
    pub circle_and_points: CenterCirclePoints<Pixel>,
}
