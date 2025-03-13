use coordinate_systems::{Camera, Robot};
use nalgebra::{vector, SVector, UnitQuaternion};
use serde::{Deserialize, Serialize};

use linear_algebra::{IntoTransform, Rotation3};
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use projection::camera_matrix::CameraMatrix;
use types::camera_position::CameraPosition;

pub const AMOUNT_OF_PARAMETERS: usize = 9;

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
)]
pub struct Corrections {
    pub correction_in_robot: Rotation3<Robot, Robot, f32>,
    pub correction_in_camera_top: Rotation3<Camera, Camera, f32>,
    pub correction_in_camera_bottom: Rotation3<Camera, Camera, f32>,
}

impl From<&SVector<f32, AMOUNT_OF_PARAMETERS>> for Corrections {
    fn from(parameters: &SVector<f32, AMOUNT_OF_PARAMETERS>) -> Self {
        Self {
            // correction_in_robot: UnitQuaternion::from_euler_angles(
            //     parameters[0],
            //     parameters[1],
            //     parameters[2],
            // )
            // .framed_transform(),
            correction_in_robot: UnitQuaternion::identity().framed_transform(),
            correction_in_camera_top: UnitQuaternion::from_euler_angles(
                parameters[3],
                parameters[4],
                parameters[5],
            )
            .framed_transform(),
            correction_in_camera_bottom: UnitQuaternion::from_euler_angles(
                parameters[6],
                parameters[7],
                parameters[8],
            )
            .framed_transform(),
        }
    }
}

impl From<&Corrections> for SVector<f32, AMOUNT_OF_PARAMETERS> {
    fn from(parameters: &Corrections) -> Self {
        let (robot_roll, robot_pitch, robot_yaw) =
            parameters.correction_in_robot.inner.euler_angles();
        let (camera_top_roll, camera_top_pitch, camera_top_yaw) =
            parameters.correction_in_camera_top.inner.euler_angles();
        let (camera_bottom_roll, camera_bottom_pitch, camera_bottom_yaw) =
            parameters.correction_in_camera_bottom.inner.euler_angles();
        vector![
            robot_roll,
            robot_pitch,
            robot_yaw,
            camera_top_roll,
            camera_top_pitch,
            camera_top_yaw,
            camera_bottom_roll,
            camera_bottom_pitch,
            camera_bottom_yaw
        ]
    }
}

pub(crate) fn get_corrected_camera_matrix(
    input_matrix: &CameraMatrix,
    position: CameraPosition,
    parameters: &Corrections,
) -> CameraMatrix {
    input_matrix.to_corrected(
        parameters.correction_in_robot,
        match position {
            CameraPosition::Top => parameters.correction_in_camera_top,
            CameraPosition::Bottom => parameters.correction_in_camera_bottom,
        },
    )
}
