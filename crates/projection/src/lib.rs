pub mod camera_matrices;
pub mod camera_matrix;
pub mod camera_projection;
pub mod horizon;
pub mod intrinsic;

use thiserror::Error;

use crate::camera_matrix::CameraMatrix;
use coordinate_systems::{Camera, Ground, Pixel, Robot};
use linear_algebra::{point, vector, Isometry3, Point2, Point3, Vector3};

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
        radius_in_ground_coordinates: f32,
        pixel_coordinates: Point2<Pixel>,
    ) -> Result<f32, Error>;
    fn is_above_horizon(&self, pixel: Point2<Pixel>, plane_height: f32) -> bool;
}

impl Projection for CameraMatrix {
    fn is_above_horizon(&self, pixel: Point2<Pixel>, plane_height: f32) -> bool {
        let bearing = self.bearing(pixel);
        let bearing_in_ground = self.ground_to_camera.inverse() * bearing;

        struct ElevatedGround;
        let ground_to_elevated_ground =
            Isometry3::<Ground, ElevatedGround>::from(vector![0., 0., -plane_height]);
        let camera =
            ground_to_elevated_ground * self.ground_to_camera.inverse().as_pose().position();

        bearing_in_ground.z() * camera.z() >= 0.0
    }

    fn camera_to_pixel(&self, camera_ray: Vector3<Camera>) -> Result<Point2<Pixel>, Error> {
        if camera_ray.z() <= 0.0 {
            return Err(Error::BehindCamera);
        }

        Ok(self.intrinsics.project(camera_ray))
    }

    fn pixel_to_ground(&self, pixel_coordinates: Point2<Pixel>) -> Result<Point2<Ground>, Error> {
        if self.is_above_horizon(pixel_coordinates, 0.0) {
            return Err(Error::NotOnProjectionPlane);
        }
        Ok(self
            .pixel_to_ground
            .back_project_unchecked(pixel_coordinates)
            .xy())
    }

    fn pixel_to_ground_with_z(
        &self,
        pixel: Point2<Pixel>,
        z: f32,
    ) -> Result<Point2<Ground>, Error> {
        if self.is_above_horizon(pixel, z) {
            return Err(Error::NotOnProjectionPlane);
        }

        let inverse_camera_matrix = self.ground_to_pixel.inverse(z);

        Ok(inverse_camera_matrix.back_project_unchecked(pixel).xy())
    }

    fn ground_to_pixel(&self, ground_coordinates: Point2<Ground>) -> Result<Point2<Pixel>, Error> {
        self.ground_with_z_to_pixel(ground_coordinates, 0.0)
    }

    fn ground_with_z_to_pixel(
        &self,
        ground_coordinates: Point2<Ground>,
        z: f32,
    ) -> Result<Point2<Pixel>, Error> {
        let ground_coordinates = point![ground_coordinates.x(), ground_coordinates.y(), z];
        let camera_ray = self.ground_to_camera * ground_coordinates;

        if camera_ray.z() <= 0.0 {
            return Err(Error::BehindCamera);
        }

        Ok(self.ground_to_pixel.project(ground_coordinates))
    }

    fn robot_to_pixel(&self, robot_coordinates: Point3<Robot>) -> Result<Point2<Pixel>, Error> {
        let robot_to_camera = self.head_to_camera * self.robot_to_head;
        let camera_coordinates = robot_to_camera * robot_coordinates;
        self.camera_to_pixel(camera_coordinates.coords())
    }

    fn get_pixel_radius(
        &self,
        radius_in_ground_coordinates: f32,
        pixel_coordinates: Point2<Pixel>,
    ) -> Result<f32, Error> {
        let ground_coordinates =
            self.pixel_to_ground_with_z(pixel_coordinates, radius_in_ground_coordinates)?;
        let ball_center = point![
            ground_coordinates.x(),
            ground_coordinates.y(),
            radius_in_ground_coordinates
        ];

        let camera_coordinates = self.ground_to_camera * ball_center;

        let angle = f32::atan2(
            radius_in_ground_coordinates,
            camera_coordinates.coords().norm(),
        );

        Ok(self.image_size.y() * angle / self.field_of_view.y)
    }

    fn bearing(&self, pixel_coordinates: Point2<Pixel>) -> Vector3<Camera> {
        self.intrinsics.bearing(pixel_coordinates)
    }
}
