use nalgebra::{matrix, Matrix3};
use thiserror::Error;

use coordinate_systems::{Camera, Ground, Pixel, Robot};
use linear_algebra::{point, vector, Point2, Point3, Vector2, Vector3};
use types::camera_matrix::CameraMatrix;

#[derive(Debug, Error)]
pub enum Error {
    #[error("position is too close to the camera to calculate")]
    TooClose,
    #[error("position is behind the camera")]
    BehindCamera,
    #[error("the pixel position cannot be projected onto the projection plane")]
    NotOnProjectionPlane,
    #[error("camera matrix is not invertible")]
    NotInvertible,
}

pub trait Projection {
    fn bearing(&self, pixel_coordinates: Point2<Pixel>) -> Vector3<Camera>;
    fn camera_to_pixel(&self, camera_ray: Vector3<Camera>) -> Result<Point2<Pixel>, Error>;
    fn pixel_to_ground(&self, pixel_coordinates: Point2<Pixel>) -> Result<Point2<Ground>, Error>;
    fn pixel_to_ground_with_z(
        &self,
        pixel_coordinates: Point2<Pixel>,
        z: f32,
    ) -> Result<Point2<Ground>, Error>;
    fn ground_to_pixel(&self, ground_coordinates: Point2<Ground>) -> Result<Point2<Pixel>, Error>;
    fn ground_with_z_to_pixel(
        &self,
        ground_coordinates: Point2<Ground>,
        z: f32,
    ) -> Result<Point2<Pixel>, Error>;
    fn robot_to_pixel(&self, robot_coordinates: Point3<Robot>) -> Result<Point2<Pixel>, Error>;
    fn get_pixel_radius(
        &self,
        radius_in_robot_coordinates: f32,
        pixel_coordinates: Point2<Pixel>,
        resolution: Point2<Pixel, u32>,
    ) -> Result<f32, Error>;
    fn camera_matrix_for_z(&self, z: f32) -> Matrix3<f32>;
}

impl Projection for CameraMatrix {
    fn camera_matrix_for_z(&self, z: f32) -> Matrix3<f32> {
        let extrinsics  = self.head_to_camera * self.robot_to_head * self.ground_to_robot;
        let total_camera_matrix =
            self.intrinsics * extrinsics.inner.to_matrix();

        let projection = matrix![
            1.0, 0.0, 0.0;
            0.0, 1.0, 0.0;
            0.0, 0.0, z;
            0.0, 0.0, 1.0;
        ];

        total_camera_matrix * projection
    }

    fn camera_to_pixel(&self, camera_ray: Vector3<Camera>) -> Result<Point2<Pixel>, Error> {
        if camera_ray.z() <= 0.0 {
            return Err(Error::BehindCamera);
        }

        let pixel_point = self.intrinsics * camera_ray.inner.to_homogeneous();

        Ok(point![
            pixel_point.x / pixel_point.z,
            pixel_point.y / pixel_point.z,
        ])
    }

    fn pixel_to_ground(&self, pixel_coordinates: Point2<Pixel>) -> Result<Point2<Ground>, Error> {
        self.pixel_to_ground_with_z(pixel_coordinates, 0.0)
    }

    fn pixel_to_ground_with_z(
        &self,
        pixel_coordinates: Point2<Pixel>,
        z: f32,
    ) -> Result<Point2<Ground>, Error> {
        let bearing = self.bearing(pixel_coordinates);

        let ground_to_camera = self.head_to_camera * self.robot_to_head * self.ground_to_robot;

        let bearing_in_ground = ground_to_camera.inverse() * bearing;
        let camera = ground_to_camera.inverse().as_pose().position();

        if bearing_in_ground.dot(camera.coords()) >= 0.0 {
            return Err(Error::NotOnProjectionPlane)
        }

        // let camera_ray = self.pixel_to_camera(pixel_coordinates);
        // let camera_to_elevated_ground = Isometry3::translation(0., 0., -z) * self.camera_to_ground;

        // let camera_position = camera_to_elevated_ground * Point3::origin();
        // let camera_ray_over_ground = camera_to_elevated_ground * camera_ray;

        // if relative_eq!(camera_ray_over_ground.z, 0.0) {
        //     return Err(Error::NotOnProjectionPlane);
        // }

        // let intersection_scalar = -camera_position.z / camera_ray_over_ground.z;

        // if intersection_scalar < 0.0 {
        //     return Err(Error::BehindCamera);
        // }

        // let intersection_point = camera_position + camera_ray_over_ground * intersection_scalar;

        // Ok(intersection_point.xy())

        let camera_matrix = self.camera_matrix_for_z(z);
        let inverse_camera_matrix = camera_matrix.try_inverse().ok_or(Error::NotInvertible)?;

        let ground = inverse_camera_matrix * pixel_coordinates.inner.to_homogeneous();

        Ok(point![ground.x, ground.y] / ground.z)
    }

    fn ground_to_pixel(&self, ground_coordinates: Point2<Ground>) -> Result<Point2<Pixel>, Error> {
        self.ground_with_z_to_pixel(ground_coordinates, 0.0)
    }

    fn ground_with_z_to_pixel(
        &self,
        ground_coordinates: Point2<Ground>,
        z: f32,
    ) -> Result<Point2<Pixel>, Error> {
        let ground_to_camera = self.head_to_camera * self.robot_to_head * self.ground_to_robot;
        let camera_ray = ground_to_camera * point![ground_coordinates.x(), ground_coordinates.y(), z];

        if camera_ray.z() <= 0.0 {
            return Err(Error::BehindCamera);
        }

        let camera_matrix = self.camera_matrix_for_z(z);
        let projected = camera_matrix * ground_coordinates.inner.to_homogeneous();

        let pixel = point![projected.x / projected.z, projected.y / projected.z];

        Ok(pixel)
    }

    fn robot_to_pixel(&self, robot_coordinates: Point3<Robot>) -> Result<Point2<Pixel>, Error> {
        let robot_to_camera = self.head_to_camera * self.robot_to_head;
        let camera_coordinates = robot_to_camera * robot_coordinates;
        self.camera_to_pixel(camera_coordinates.coords())
    }

    fn get_pixel_radius(
        &self,
        radius_in_robot_coordinates: f32,
        pixel_coordinates: Point2<Pixel>,
        resolution: Point2<Pixel, u32>,
    ) -> Result<f32, Error> {
        let robot_coordinates =
            self.pixel_to_ground_with_z(pixel_coordinates, radius_in_robot_coordinates)?;
        let camera_coordinates =
            self.ground_to_robot * point![robot_coordinates.x(), robot_coordinates.y(), 0.0];
        let distance = camera_coordinates.coords().norm();
        if distance <= radius_in_robot_coordinates {
            return Err(Error::TooClose);
        }
        let angle = (radius_in_robot_coordinates / distance).asin();
        Ok(resolution.y() as f32 * angle / self.field_of_view.y)
    }
}
