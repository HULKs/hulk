use nalgebra::{point, vector, Isometry3, Point2, UnitQuaternion, Vector2, Vector3};
use serde::{Deserialize, Serialize};

use crate::{
    ground_calculation::Ground, robot_dimensions::RobotDimensions,
    robot_kinematics::RobotKinematics,
};

pub enum CameraPosition {
    Top,
    Bottom,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CameraMatrix {
    pub camera_to_ground: Isometry3<f32>,
    pub ground_to_camera: Isometry3<f32>,
    pub camera_to_robot: Isometry3<f32>,
    pub robot_to_camera: Isometry3<f32>,
    pub focal_length: Vector2<f32>,
    pub optical_center: Point2<f32>,
    pub field_of_view: Vector2<f32>,
    pub horizon: Horizon,
}

impl Default for CameraMatrix {
    fn default() -> Self {
        Self {
            camera_to_ground: Isometry3::identity(),
            ground_to_camera: Isometry3::identity(),
            camera_to_robot: Isometry3::identity(),
            robot_to_camera: Isometry3::identity(),
            focal_length: Default::default(),
            optical_center: Point2::origin(),
            field_of_view: Default::default(),
            horizon: Default::default(),
        }
    }
}

impl CameraMatrix {
    pub fn from_ground_and_robot_kinematics(
        camera_position: CameraPosition,
        ground: Ground,
        robot_kinematics: RobotKinematics,
        extrinsic_rotation: Vector3<f32>,
        focal_length: Vector2<f32>,
        optical_center: Vector2<f32>,
    ) -> Self {
        let camera_to_head = Self::camera_to_head(camera_position, extrinsic_rotation);
        let camera_to_robot = robot_kinematics.head_to_robot * camera_to_head;
        let camera_to_ground = ground.robot_to_ground * camera_to_robot;
        Self::camera_matrix_for_camera(
            camera_to_robot,
            camera_to_ground,
            focal_length,
            optical_center,
        )
    }

    fn camera_to_head(
        camera_position: CameraPosition,
        extrinsic_rotation: Vector3<f32>,
    ) -> Isometry3<f32> {
        let extrinsic_angles_in_radians = extrinsic_rotation.map(|a: f32| a.to_radians());
        let extrinsic_rotation = UnitQuaternion::from_euler_angles(
            extrinsic_angles_in_radians.x,
            extrinsic_angles_in_radians.y,
            extrinsic_angles_in_radians.z,
        );
        let neck_to_camera = match camera_position {
            CameraPosition::Top => RobotDimensions::NECK_TO_TOP_CAMERA,
            CameraPosition::Bottom => RobotDimensions::NECK_TO_BOTTOM_CAMERA,
        };
        let camera_pitch = match camera_position {
            CameraPosition::Top => 1.2f32.to_radians(),
            CameraPosition::Bottom => 39.7f32.to_radians(),
        };
        Isometry3::from(neck_to_camera)
            * Isometry3::rotation(Vector3::y() * camera_pitch)
            * extrinsic_rotation
    }

    fn camera_matrix_for_camera(
        camera_to_robot: Isometry3<f32>,
        camera_to_ground: Isometry3<f32>,
        focal_length: Vector2<f32>,
        optical_center: Vector2<f32>,
    ) -> CameraMatrix {
        // Calculate FOV using;
        // fov_x = 2 * atan(image_width/ (2 * focal_lengths_x)) -> same for fov_y.
        // https://www.edmundoptics.eu/knowledge-center/application-notes/imaging/understanding-focal-length-and-field-of-view/
        // focal_lengths & cc_optical_center in [0, 1] range & assuming image_size -> 1.0
        let field_of_view = focal_length.map(|f| 2.0 * (0.5 / f).atan());

        let image_width = 640;
        let image_height = 480;
        let focal_length = vector![
            focal_length.x * (image_width as f32),
            focal_length.y * (image_height as f32)
        ];
        let optical_center = point![
            optical_center.x * (image_width as f32),
            optical_center.y * (image_height as f32)
        ];

        let rotation_matrix = camera_to_ground.rotation.to_rotation_matrix();
        let horizon_slope_is_infinite = rotation_matrix[(2, 2)] == 0.0;
        let horizon = if horizon_slope_is_infinite {
            Horizon {
                left_horizon_y: 0.0,
                right_horizon_y: 0.0,
            }
        } else {
            let left_horizon_y = optical_center.y
                + focal_length.y
                    * (rotation_matrix[(2, 0)]
                        + optical_center.x * rotation_matrix[(2, 1)] / focal_length.x)
                    / rotation_matrix[(2, 2)];
            let slope = -focal_length.y * rotation_matrix[(2, 1)]
                / (focal_length.x * rotation_matrix[(2, 2)]);
            let right_horizon_y = left_horizon_y + (slope * ((image_width - 1) as f32));

            Horizon {
                left_horizon_y,
                right_horizon_y,
            }
        };

        CameraMatrix {
            camera_to_ground,
            ground_to_camera: camera_to_ground.inverse(),
            camera_to_robot,
            robot_to_camera: camera_to_robot.inverse(),
            focal_length,
            optical_center,
            field_of_view,
            horizon,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Horizon {
    pub left_horizon_y: f32,
    pub right_horizon_y: f32,
}
