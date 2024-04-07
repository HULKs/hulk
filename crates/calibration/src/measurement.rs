use coordinate_systems::Pixel;
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use projection::camera_matrix::CameraMatrix;
use serde::{Deserialize, Serialize};
use types::camera_position::CameraPosition;

use crate::lines::Lines;

#[derive(
    Clone, Debug, Default, Serialize, Deserialize, PathSerialize, PathIntrospect, PathDeserialize,
)]
pub struct Measurement {
    pub position: CameraPosition,
    pub matrix: CameraMatrix,
    pub lines: Lines<Pixel>,
}
