use serde::{Deserialize, Serialize};

use coordinate_systems::Pixel;
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use projection::camera_matrix::CameraMatrix;

use super::lines::Lines;

#[derive(
    Clone, Debug, Default, Serialize, Deserialize, PathSerialize, PathIntrospect, PathDeserialize,
)]
pub struct Measurement {
    pub matrix: CameraMatrix,
    pub lines: Lines<Pixel>,
}
