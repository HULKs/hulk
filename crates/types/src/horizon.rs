use coordinate_systems::{Camera, Ground, Pixel};
use linear_algebra::{point, vector, Isometry3, Point2, Vector2, Vector3};
use nalgebra::Matrix3x4;
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize, SerializeHierarchy)]
pub struct Horizon {
    pub point_on_horizon: Point2<Pixel>,
    pub horizon_normal: Vector2<Pixel>,
}

impl Horizon {
    pub fn horizon_y_minimum(&self) -> f32 {
        self.y_at_x(0.0).min(self.y_at_x(640.0))
    }

    pub fn y_at_x(&self, x: f32) -> f32 {
        let normal = self.horizon_normal;
        let point = self.point_on_horizon;

        -normal.x() * (x - point.x()) / normal.y() + point.y()
    }

    pub fn from_parameters(
        camera_to_ground: Isometry3<Camera, Ground>,
        intrinsics: Matrix3x4<f32>,
    ) -> Self {
        let horizon_normal = Vector3::z_axis();
        let horizon_normal_camera = camera_to_ground.inverse() * horizon_normal;
        let horizon_normal_camera: Vector3<Pixel> =
            vector![horizon_normal_camera.x(), horizon_normal_camera.y(), 0.0].normalize();
        let horizon_normal_image = intrinsics * horizon_normal_camera.inner.to_homogeneous();

        let camera_front = Vector3::z_axis();
        let ground_front = camera_to_ground * camera_front;
        let ground_front = vector![ground_front.x(), ground_front.y(), 0.0].normalize();

        let horizon_point_camera = camera_to_ground.inverse() * ground_front;
        let horizon_point_image = intrinsics * horizon_point_camera.inner.to_homogeneous();

        Self {
            point_on_horizon: point![horizon_point_image.x, horizon_point_image.y],
            horizon_normal: vector![horizon_normal_image.x, horizon_normal_image.y],
        }
    }
}
