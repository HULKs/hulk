//! Converts spatial targets and image framing into desired joint positions.
//! Target selection, temporal patterns, and motor control belong to other modules.

use coordinate_systems::{Ground, Robot};
use kinematics::joints::head::HeadJoints;
use linear_algebra::{Isometry3, Point3};
use projection::camera_matrix::CameraMatrix;
use types::motion_command::ImageRegion;

use crate::parameters::Parameters;

pub struct GazeGeometry<'a> {
    pub camera_matrix: &'a CameraMatrix,
    pub ground_to_robot: Isometry3<Ground, Robot>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LookAtError {
    InvalidTarget,
    InvalidGeometry,
}

/// Ground-plane requests use z = 0; elevated targets use the same framing interface.
pub fn look_at(
    _target: Point3<Ground>,
    _image_region: ImageRegion,
    _geometry: &GazeGeometry<'_>,
    _parameters: &Parameters,
) -> Result<HeadJoints<f32>, LookAtError> {
    todo!("solve K1 gaze geometry while honoring the requested image region")
}
