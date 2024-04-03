use coordinate_systems::{Camera, Ground, Pixel};
use linear_algebra::{vector, Isometry3, Point2, Vector2, Vector3};
use nalgebra::Matrix3x4;
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize, SerializeHierarchy)]
pub struct Horizon {
    pub vanishing_point: Point2<Pixel>,
    pub normal: Vector2<Pixel>,
}

impl Horizon {
    pub fn horizon_y_minimum(&self) -> f32 {
        self.y_at_x(0.0).min(self.y_at_x(640.0))
    }

    pub fn y_at_x(&self, x: f32) -> f32 {
        -self.normal.x() * (x - self.vanishing_point.x()) / self.normal.y() + self.vanishing_point.y()
    }

    fn find_vanishing_point(ground_to_camera: Isometry3<Ground, Camera>, intrinsics: Matrix3x4<f32>) -> Point2<Pixel> {
        let camera_front = Vector3::z_axis();
        let ground_front = ground_to_camera.inverse() * camera_front;
        let ground_front = vector![ground_front.x(), ground_front.y(), 0.0].normalize();

        let vanishing_point = ground_to_camera * ground_front;
        let vanishing_point_image = intrinsics * vanishing_point.inner.to_homogeneous();

        Point2::from(vanishing_point_image.xy())
    }

    fn find_horizon_normal(ground_to_camera: Isometry3<Ground, Camera>, intrinsics: Matrix3x4<f32>) -> Vector2<Pixel> {
        let up = Vector3::z_axis();
        let up_in_camera = ground_to_camera * up;
        let horizon_normal_camera: Vector3<Pixel> =
            vector![up_in_camera.x(), up_in_camera.y(), 0.0].normalize();
        let horizon_normal_image = intrinsics * horizon_normal_camera.inner.to_homogeneous();

        Vector2::wrap(horizon_normal_image.xy())
    }

    pub fn from_parameters(
        ground_to_camera: Isometry3<Ground, Camera>,
        intrinsics: Matrix3x4<f32>,
    ) -> Self {

        Self {
            vanishing_point: Self::find_vanishing_point(ground_to_camera, intrinsics),
            normal: Self::find_horizon_normal(ground_to_camera, intrinsics),
        }
    }
}
