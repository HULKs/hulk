use coordinate_systems::{Camera, Pixel};
use linear_algebra::{point, Isometry3, Point2, Point3, Transform};

use crate::intrinsic::Intrinsic;

pub struct CameraProjection<From> {
    extrinsic: Isometry3<From, Camera>,
    intrinsic: Intrinsic,
}

impl<From> CameraProjection<From> {
    pub fn new(extrinsic: Isometry3<From, Camera>, intrinsic: Intrinsic) -> Self {
        Self {
            extrinsic,
            intrinsic,
        }
    }

    pub fn project(&self, point: Point3<From>) -> Point2<Pixel> {
        let point_in_camera = self.extrinsic * point;
        self.intrinsic.project(point_in_camera.coords())
    }

    pub fn inverse(&self, z: f32) -> InverseCameraProjection<From> {
        InverseCameraProjection::from(self, z)
    }
}

pub struct InverseCameraProjection<To> {
    back_project: Transform<Pixel, To, nalgebra::Matrix3<f32>>,
    z: f32,
}

impl<To> InverseCameraProjection<To> {
    fn from(forward: &CameraProjection<To>, z: f32) -> Self {
        let projection_matrix =
            forward.intrinsic.as_matrix() * forward.extrinsic.inner.to_homogeneous();
        let z_projection = nalgebra::matrix![
            1.0, 0.0, 0.0;
            0.0, 1.0, 0.0;
            0.0, 0.0, z;
            0.0, 0.0, 1.0;
        ];
        let inverse = (projection_matrix * z_projection)
            .try_inverse()
            .expect("camera matrix is not invertible");

        Self {
            back_project: Transform::wrap(inverse),
            z,
        }
    }

    pub fn back_project(&self, point: Point2<Pixel>) -> Point3<To> {
        let point_to = self.back_project.inner * point.inner.to_homogeneous();
        point![point_to.x / point_to.z, point_to.y / point_to.z, self.z]
    }
}
