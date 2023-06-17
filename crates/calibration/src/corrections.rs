use nalgebra::{vector, Rotation3, SVector};

pub const AMOUNT_OF_PARAMETERS: usize = 9;

#[derive(Clone, Copy, Debug, Default)]
pub struct Corrections {
    pub correction_in_robot: Rotation3<f32>,
    pub correction_in_camera_top: Rotation3<f32>,
    pub correction_in_camera_bottom: Rotation3<f32>,
}

impl From<&SVector<f32, AMOUNT_OF_PARAMETERS>> for Corrections {
    fn from(parameters: &SVector<f32, AMOUNT_OF_PARAMETERS>) -> Self {
        Self {
            correction_in_robot: Rotation3::from_euler_angles(
                parameters[0],
                parameters[1],
                parameters[2],
            ),
            correction_in_camera_top: Rotation3::from_euler_angles(
                parameters[3],
                parameters[4],
                parameters[5],
            ),
            correction_in_camera_bottom: Rotation3::from_euler_angles(
                parameters[6],
                parameters[7],
                parameters[8],
            ),
        }
    }
}

impl From<&Corrections> for SVector<f32, AMOUNT_OF_PARAMETERS> {
    fn from(parameters: &Corrections) -> Self {
        let (robot_roll, robot_pitch, robot_yaw) = parameters.correction_in_robot.euler_angles();
        let (camera_top_roll, camera_top_pitch, camera_top_yaw) =
            parameters.correction_in_camera_top.euler_angles();
        let (camera_bottom_roll, camera_bottom_pitch, camera_bottom_yaw) =
            parameters.correction_in_camera_bottom.euler_angles();
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
