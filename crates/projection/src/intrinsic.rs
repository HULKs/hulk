use coordinate_systems::{Camera, NormalizedDeviceCoordinates, Pixel};
use linear_algebra::{point, vector, Point2, Vector3};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, SerializeHierarchy)]
pub struct Intrinsic {
    focals: nalgebra::Vector2<f32>,
    optical_center: Point2<Pixel>,
}

impl Default for Intrinsic {
    fn default() -> Self {
        Self {
            focals: nalgebra::vector![1.0, 1.0],
            optical_center: point![0.0, 0.0],
        }
    }
}

impl Intrinsic {
    pub fn new(focal_length: nalgebra::Vector2<f32>, optical_center: Point2<Pixel>) -> Self {
        Self {
            focals: focal_length,
            optical_center,
        }
    }

    pub fn as_matrix(&self) -> nalgebra::Matrix3x4<f32> {
        nalgebra::matrix![
            self.focals.x, 0.0, self.optical_center.x(), 0.0;
            0.0, self.focals.y, self.optical_center.y(), 0.0;
            0.0, 0.0, 1.0, 0.0;
        ]
    }

    pub fn transform(&self, ray: Vector3<Camera>) -> Vector3<NormalizedDeviceCoordinates> {
        let (x, y, z) = (ray.x(), ray.y(), ray.z());

        vector![
            self.focals.x * x + self.optical_center.x() * z,
            self.focals.y * y + self.optical_center.y() * z,
            z
        ]
    }

    pub fn project(&self, ray: Vector3<Camera>) -> Point2<Pixel> {
        let projected = self.transform(ray);
        point![projected.x() / projected.z(), projected.y() / projected.z()]
    }

    pub fn bearing(&self, pixel: Point2<Pixel>) -> Vector3<Camera> {
        let x = (pixel.x() - self.optical_center.x()) / self.focals.x;
        let y = (pixel.y() - self.optical_center.y()) / self.focals.y;

        vector![x, y, 1.0]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intrinsic_projection() {
        let intrinsic = Intrinsic::new(nalgebra::vector![2.0, 2.0], point![1.0, 1.0]);
        let point = vector![0.0, 0.0, 1.0];
        let projected = intrinsic.project(point);
        assert_eq!(projected, point![1.0, 1.0]);
    }

    #[test]
    fn intrinsic_bearing() {
        let intrinsic = Intrinsic::new(nalgebra::vector![2.0, 2.0], point![1.0, 1.0]);
        let pixel = point![1.0, 1.0];
        let bearing = intrinsic.bearing(pixel);
        assert_eq!(bearing, vector![0.0, 0.0, 1.0]);
    }
}
