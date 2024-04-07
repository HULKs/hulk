use std::ops::Index;

use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;
use types::camera_position::CameraPosition;

use crate::camera_matrix::CameraMatrix;

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct CameraMatrices {
    pub top: CameraMatrix,
    pub bottom: CameraMatrix,
}

impl Index<CameraPosition> for CameraMatrices {
    type Output = CameraMatrix;

    fn index(&self, index: CameraPosition) -> &Self::Output {
        match index {
            CameraPosition::Top => &self.top,
            CameraPosition::Bottom => &self.bottom,
        }
    }
}