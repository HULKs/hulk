use coordinate_systems::{Camera, Ground, Head, Pixel, Robot};
use linear_algebra::{IntoFramed, Isometry3, Rotation3, Vector2};
use ros2::sensor_msgs::camera_info::CameraInfo;
use serde::{Deserialize, Serialize};

use crate::{
    camera_projection::{CameraProjection, InverseCameraProjection},
    horizon::Horizon,
    intrinsic::Intrinsic,
};

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize, ros_z::Message)]
pub struct CameraMatrix {
    pub ground_to_robot: Isometry3<Ground, Robot>,
    pub robot_to_head: Isometry3<Robot, Head>,
    pub head_to_camera: Isometry3<Head, Camera>,
    pub intrinsics: Intrinsic,
    pub field_of_view: nalgebra::Vector2<f32>,
    pub horizon: Option<Horizon>,
    pub image_size: Vector2<Pixel>,

    // Precomputed values for faster calculations
    pub ground_to_camera: Isometry3<Ground, Camera>,

    pub ground_to_pixel: CameraProjection<Ground>,
    pub pixel_to_ground: InverseCameraProjection<Ground>,
}

impl CameraMatrix {
    /// This takes [0, 1] range focal length & optical center values & actual image size to create camera matrix.
    pub fn from_normalized_focal_and_center(
        focal_length: nalgebra::Vector2<f32>,
        optical_center: nalgebra::Point2<f32>,
        image_size: Vector2<Pixel>,
        ground_to_robot: Isometry3<Ground, Robot>,
        robot_to_head: Isometry3<Robot, Head>,
        head_to_camera: Isometry3<Head, Camera>,
    ) -> Self {
        let focal_length_scaled = focal_length.component_mul(&image_size.inner);
        let optical_center_scaled = optical_center
            .coords
            .component_mul(&image_size.inner)
            .framed()
            .as_point();

        let intrinsics = Intrinsic::new(focal_length_scaled, optical_center_scaled);
        let field_of_view = Intrinsic::calculate_field_of_view(intrinsics.focals, image_size);

        let ground_to_camera = head_to_camera * robot_to_head * ground_to_robot;
        let horizon = Horizon::from_parameters(ground_to_camera, &intrinsics);

        Self {
            intrinsics,
            field_of_view,
            horizon,
            ground_to_robot,
            robot_to_head,
            head_to_camera,
            image_size,
            // Precomputed values
            ground_to_camera,
            ground_to_pixel: CameraProjection::new(ground_to_camera, intrinsics),
            pixel_to_ground: CameraProjection::new(ground_to_camera, intrinsics).inverse(0.0),
        }
    }

    pub fn from_camera_info(
        camera_info: &CameraInfo,
        image_size: Vector2<Pixel>,
        ground_to_robot: Isometry3<Ground, Robot>,
        robot_to_head: Isometry3<Robot, Head>,
        head_to_camera: Isometry3<Head, Camera>,
    ) -> Self {
        let intrinsics = Intrinsic::from(camera_info);
        let field_of_view = Intrinsic::calculate_field_of_view(intrinsics.focals, image_size);

        let ground_to_camera = head_to_camera * robot_to_head * ground_to_robot;
        let horizon = Horizon::from_parameters(ground_to_camera, &intrinsics);

        Self {
            intrinsics,
            field_of_view,
            horizon,
            ground_to_robot,
            robot_to_head,
            head_to_camera,
            image_size,
            // Precomputed values
            ground_to_camera,
            ground_to_pixel: CameraProjection::new(ground_to_camera, intrinsics),
            pixel_to_ground: CameraProjection::new(ground_to_camera, intrinsics).inverse(0.0),
        }
    }

    pub fn compute_memoized(&mut self) {
        self.ground_to_camera = self.head_to_camera * self.robot_to_head * self.ground_to_robot;
        self.ground_to_pixel = CameraProjection::new(self.ground_to_camera, self.intrinsics);
        self.pixel_to_ground =
            CameraProjection::new(self.ground_to_camera, self.intrinsics).inverse(0.0);
    }

    pub fn to_corrected(
        &self,
        correction_in_robot: Rotation3<Robot, Robot>,
        correction_in_camera: Rotation3<Camera, Camera>,
    ) -> Self {
        let corrected_ground_to_robot = self.ground_to_robot;
        let corrected_robot_to_head = self.robot_to_head * correction_in_robot;
        let corrected_head_to_camera = correction_in_camera * self.head_to_camera;

        let corrected_ground_to_camera =
            corrected_head_to_camera * corrected_robot_to_head * corrected_ground_to_robot;

        let new_horizon = Horizon::from_parameters(corrected_ground_to_camera, &self.intrinsics);

        let ground_to_pixel = CameraProjection::new(corrected_ground_to_camera, self.intrinsics);
        let ground_to_pixel = ground_to_pixel.clone();

        Self {
            ground_to_robot: corrected_ground_to_robot,
            robot_to_head: corrected_robot_to_head,
            head_to_camera: corrected_head_to_camera,
            intrinsics: self.intrinsics,
            field_of_view: self.field_of_view,
            horizon: new_horizon,
            image_size: self.image_size,
            ground_to_camera: corrected_ground_to_camera,
            ground_to_pixel: ground_to_pixel.clone(),
            pixel_to_ground: ground_to_pixel.inverse(0.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;
    use linear_algebra::{Orientation3, vector};

    use super::*;

    #[test]
    fn check_field_of_view_calculation() {
        // Old implementation, assumes normalized values
        fn old_fov(focal_lengths: nalgebra::Vector2<f32>) -> nalgebra::Vector2<f32> {
            focal_lengths.map(|f| 2.0 * (0.5 / f).atan())
        }

        let focals = nalgebra::vector![0.63, 1.34];
        let image_size = vector![1.0, 1.0];
        let image_size_abs = vector![640.0, 480.0];

        let focals_scaled = image_size_abs
            .inner
            .zip_map(&focals, |dim, focal| dim * focal);

        assert_relative_eq!(
            old_fov(focals),
            Intrinsic::calculate_field_of_view(focals, image_size)
        );

        assert_relative_eq!(
            old_fov(focals),
            Intrinsic::calculate_field_of_view(focals_scaled, image_size_abs)
        );
    }

    #[test]
    fn correction_in_robot_is_applied_once_without_changing_ground_to_robot() {
        let ground_to_robot = Isometry3::from_parts(
            vector![<Robot>, 0.1, -0.2, -0.5],
            Orientation3::from_euler_angles(0.03, -0.04, 0.0),
        );
        let robot_to_head = Isometry3::from_translation(0.0, 0.0, 0.3);
        let head_to_camera = Isometry3::from_translation(0.05, 0.0, 0.02);
        let camera_matrix = CameraMatrix::from_normalized_focal_and_center(
            nalgebra::vector![0.5, 0.5],
            nalgebra::point![0.5, 0.5],
            vector![640.0, 480.0],
            ground_to_robot,
            robot_to_head,
            head_to_camera,
        );
        let correction_in_robot = Rotation3::from_euler_angles(0.1, -0.2, 0.3);
        let correction_in_camera = Rotation3::from_euler_angles(-0.4, 0.5, -0.6);

        let corrected = camera_matrix.to_corrected(correction_in_robot, correction_in_camera);
        let expected_ground_to_camera = correction_in_camera
            * head_to_camera
            * robot_to_head
            * correction_in_robot
            * ground_to_robot;

        assert_isometry_near(corrected.ground_to_robot, ground_to_robot);
        assert_isometry_near(corrected.ground_to_camera, expected_ground_to_camera);
    }

    fn assert_isometry_near<From, To>(actual: Isometry3<From, To>, expected: Isometry3<From, To>) {
        assert!(
            (actual.inner.translation.vector - expected.inner.translation.vector).norm() < 1.0e-6
        );
        assert!(actual.inner.rotation.angle_to(&expected.inner.rotation) < 1.0e-6);
    }
}
