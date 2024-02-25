use approx::relative_eq;
use nalgebra::{point, Isometry3, Point2, Point3, Vector2, Vector3};
use thiserror::Error;
use types::camera_matrix::CameraMatrix;

#[derive(Debug, Error)]
pub enum Error {
    #[error("position is too close to the camera to calculate")]
    TooClose,
    #[error("position is behind the camera")]
    BehindCamera,
    #[error("the pixel position cannot be projected onto the projection plane")]
    NotOnProjectionPlane,
}

pub trait Projection {
    fn pixel_to_camera(&self, pixel_coordinates: Point2<f32>) -> Vector3<f32>;
    fn camera_to_pixel(&self, camera_ray: Vector3<f32>) -> Result<Point2<f32>, Error>;
    fn pixel_to_ground(&self, pixel_coordinates: Point2<f32>) -> Result<Point2<f32>, Error>;
    fn pixel_to_ground_with_z(
        &self,
        pixel_coordinates: Point2<f32>,
        z: f32,
    ) -> Result<Point2<f32>, Error>;
    fn ground_to_pixel(&self, ground_coordinates: Point2<f32>) -> Result<Point2<f32>, Error>;
    fn ground_with_z_to_pixel(
        &self,
        ground_coordinates: Point2<f32>,
        z: f32,
    ) -> Result<Point2<f32>, Error>;
    fn pixel_to_robot_with_x(
        &self,
        pixel_coordinates: Point2<f32>,
        x: f32,
    ) -> Result<Point3<f32>, Error>;
    fn robot_to_pixel(&self, robot_coordinates: Point3<f32>) -> Result<Point2<f32>, Error>;
    fn get_pixel_radius(
        &self,
        radius_in_robot_coordinates: f32,
        pixel_coordinates: Point2<f32>,
        resolution: Vector2<u32>,
    ) -> Result<f32, Error>;
}

impl Projection for CameraMatrix {
    fn pixel_to_camera(&self, pixel_coordinates: Point2<f32>) -> Vector3<f32> {
        self.intrinisic_pixel_to_camera * pixel_coordinates.to_homogeneous()
    }

    fn camera_to_pixel(&self, camera_ray: Vector3<f32>) -> Result<Point2<f32>, Error> {
        if camera_ray.x <= 0.0 {
            return Err(Error::BehindCamera);
        }

        let pixel_point = self.intrinsic_camera_to_pixel * camera_ray;

        Ok(point![
            pixel_point.x / pixel_point.z,
            pixel_point.y / pixel_point.z,
        ])
    }

    fn pixel_to_ground(&self, pixel_coordinates: Point2<f32>) -> Result<Point2<f32>, Error> {
        self.pixel_to_ground_with_z(pixel_coordinates, 0.0)
    }

    fn pixel_to_ground_with_z(
        &self,
        pixel_coordinates: Point2<f32>,
        z: f32,
    ) -> Result<Point2<f32>, Error> {
        let camera_ray = self.pixel_to_camera(pixel_coordinates);
        let camera_to_elevated_ground = Isometry3::translation(0., 0., -z) * self.camera_to_ground;

        let camera_position = camera_to_elevated_ground * Point3::origin();
        let camera_ray_over_ground = camera_to_elevated_ground * camera_ray;

        if relative_eq!(camera_ray_over_ground.z, 0.0) {
            return Err(Error::NotOnProjectionPlane);
        }

        let intersection_scalar = -camera_position.z / camera_ray_over_ground.z;

        if intersection_scalar < 0.0 {
            return Err(Error::BehindCamera);
        }

        let intersection_point = camera_position + camera_ray_over_ground * intersection_scalar;

        Ok(intersection_point.xy())
    }

    fn ground_to_pixel(&self, ground_coordinates: Point2<f32>) -> Result<Point2<f32>, Error> {
        self.ground_with_z_to_pixel(ground_coordinates, 0.0)
    }

    fn ground_with_z_to_pixel(
        &self,
        ground_coordinates: Point2<f32>,
        z: f32,
    ) -> Result<Point2<f32>, Error> {
        self.camera_to_pixel(
            (self.ground_to_camera * point![ground_coordinates.x, ground_coordinates.y, z]).coords,
        )
    }

    fn pixel_to_robot_with_x(
        &self,
        pixel_coordinates: Point2<f32>,
        x: f32,
    ) -> Result<Point3<f32>, Error> {
        if x <= 0.0 {
            return Err(Error::BehindCamera);
        }

        let camera_ray = self.pixel_to_camera(pixel_coordinates);
        let camera_ray_over_robot = self.camera_to_robot.rotation * camera_ray;

        let distance_to_plane = x - self.camera_to_robot.translation.x;
        let slope = distance_to_plane / camera_ray_over_robot.x;

        let intersection_point =
            self.camera_to_robot.translation.vector + camera_ray_over_robot * slope;
        Ok(point![x, intersection_point.y, intersection_point.z])
    }

    fn robot_to_pixel(&self, robot_coordinates: Point3<f32>) -> Result<Point2<f32>, Error> {
        let camera_coordinates = self.robot_to_camera * robot_coordinates;
        self.camera_to_pixel(camera_coordinates.coords)
    }

    fn get_pixel_radius(
        &self,
        radius_in_robot_coordinates: f32,
        pixel_coordinates: Point2<f32>,
        resolution: Vector2<u32>,
    ) -> Result<f32, Error> {
        let robot_coordinates =
            self.pixel_to_ground_with_z(pixel_coordinates, radius_in_robot_coordinates)?;
        let camera_coordinates =
            self.ground_to_camera * point![robot_coordinates.x, robot_coordinates.y, 0.0];
        let distance = camera_coordinates.coords.norm();
        if distance <= radius_in_robot_coordinates {
            return Err(Error::TooClose);
        }
        let angle = (radius_in_robot_coordinates / distance).asin();
        Ok(resolution.y as f32 * angle / self.field_of_view.y)
    }
}
