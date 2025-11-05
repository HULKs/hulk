use nalgebra::{vector, Rotation3, SVector, UnitQuaternion};
use serde::{Deserialize, Serialize};

use approx_derive::{AbsDiffEq, RelativeEq};
use linear_algebra::IntoTransform;
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use projection::camera_matrix::CameraMatrix;

pub const AMOUNT_OF_PARAMETERS: usize = 6;

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Serialize,
    Deserialize,
    PathDeserialize,
    PathSerialize,
    PathIntrospect,
    PartialEq,
    AbsDiffEq,
    RelativeEq,
)]
#[abs_diff_eq(epsilon_type = f32)]
pub struct Corrections {
    pub correction_in_robot: Rotation3<f32>,
    pub correction_in_camera: Rotation3<f32>,
}

impl From<&SVector<f32, AMOUNT_OF_PARAMETERS>> for Corrections {
    fn from(parameters: &SVector<f32, AMOUNT_OF_PARAMETERS>) -> Self {
        Self {
            correction_in_robot: Rotation3::from_euler_angles(
                parameters[0],
                parameters[1],
                parameters[2],
            ),
            correction_in_camera: Rotation3::from_euler_angles(
                parameters[3],
                parameters[4],
                parameters[5],
            ),
        }
    }
}

impl From<&Corrections> for SVector<f32, AMOUNT_OF_PARAMETERS> {
    fn from(parameters: &Corrections) -> Self {
        let (robot_roll, robot_pitch, robot_yaw) = parameters.correction_in_robot.euler_angles();
        let (camera_roll, camera_pitch, camera_yaw) =
            parameters.correction_in_camera.euler_angles();
        vector![
            robot_roll,
            robot_pitch,
            robot_yaw,
            camera_roll,
            camera_pitch,
            camera_yaw,
        ]
    }
}

pub(crate) fn get_corrected_camera_matrix(
    input_matrix: &CameraMatrix,
    parameters: &Corrections,
) -> CameraMatrix {
    input_matrix.to_corrected(
        UnitQuaternion::from_rotation_matrix(&parameters.correction_in_robot).framed_transform(),
        UnitQuaternion::from_rotation_matrix(&parameters.correction_in_camera).framed_transform(),
    )
}
