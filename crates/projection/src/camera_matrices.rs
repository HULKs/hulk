use std::ops::Index;

use coordinate_systems::{Camera, Robot};
use linear_algebra::Rotation3;
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};
use types::camera_position::CameraPosition;

use crate::camera_matrix::CameraMatrix;

#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
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

impl CameraMatrices {
    pub fn to_corrected(
        self,
        correction_in_robot: Rotation3<Robot, Robot>,
        correction_in_camera_top: Rotation3<Camera, Camera>,
        correction_in_camera_bottom: Rotation3<Camera, Camera>,
    ) -> Self {
        Self {
            top: self
                .top
                .to_corrected(correction_in_robot, correction_in_camera_top),
            bottom: self
                .bottom
                .to_corrected(correction_in_robot, correction_in_camera_bottom),
        }
    }
}
