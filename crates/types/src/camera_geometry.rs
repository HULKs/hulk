use coordinate_systems::{Camera, Robot};
use linear_algebra::Isometry3;
use projection::{camera_matrix::CameraMatrix, intrinsic::Intrinsic};
use serde::{Deserialize, Serialize};

use ros_z::Message;

/// Calibrated camera geometry relative to the body, independent of ground contact.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize, Message)]
pub struct CameraGeometry {
    pub robot_to_camera: Isometry3<Robot, Camera>,
    pub intrinsics: Intrinsic,
}

impl From<&CameraMatrix> for CameraGeometry {
    fn from(camera: &CameraMatrix) -> Self {
        Self {
            robot_to_camera: camera.head_to_camera * camera.robot_to_head,
            intrinsics: camera.intrinsics,
        }
    }
}
