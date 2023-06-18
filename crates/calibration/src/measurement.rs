use types::CameraMatrices;

use crate::lines::Lines;

#[derive(Clone)]
pub struct Measurement {
    pub matrices: CameraMatrices,
    pub lines: Lines,
}
