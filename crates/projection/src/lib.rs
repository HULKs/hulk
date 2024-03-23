use approx::relative_eq;
use thiserror::Error;

use coordinate_systems::{Camera, Ground, Pixel, Robot};
use linear_algebra::{point, vector, IntoFramed, Isometry3, Point, Point2, Point3, Vector3};
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
    fn pixel_to_camera(&self, pixel_coordinates: Point2<Pixel>) -> Vector3<Camera>;
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
    fn pixel_to_robot_with_x(
        &self,
        pixel_coordinates: Point2<Pixel>,
        x: f32,
    ) -> Result<Point3<Robot>, Error>;
    fn robot_to_pixel(&self, robot_coordinates: Point3<Robot>) -> Result<Point2<Pixel>, Error>;
    fn get_pixel_radius(
        &self,
        radius_in_robot_coordinates: f32,
        pixel_coordinates: Point2<Pixel>,
        resolution: Point2<Pixel, u32>,
    ) -> Result<f32, Error>;
}

impl Projection for CameraMatrix {
    fn pixel_to_camera(&self, pixel_coordinates: Point2<Pixel>) -> Vector3<Camera> {
        (self.intrinisic_pixel_to_camera * pixel_coordinates.inner.to_homogeneous()).framed()
    }

    fn camera_to_pixel(&self, camera_ray: Vector3<Camera>) -> Result<Point2<Pixel>, Error> {
        if camera_ray.x() <= 0.0 {
            return Err(Error::BehindCamera);
        }

        let pixel_point = self.intrinsic_camera_to_pixel * camera_ray.inner;

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
        struct ElevatedGround;

        let camera_ray = self.pixel_to_camera(pixel_coordinates);
        let camera_to_elevated_ground =
            Isometry3::<Ground, ElevatedGround>::from(vector![0., 0., -z]) * self.camera_to_ground;

        let camera_position = Point::from(camera_to_elevated_ground.translation());
        let camera_ray_over_ground = camera_to_elevated_ground * camera_ray;

        if relative_eq!(camera_ray_over_ground.z(), 0.0) {
            return Err(Error::NotOnProjectionPlane);
        }

        let intersection_scalar = -camera_position.z() / camera_ray_over_ground.z();

        if intersection_scalar < 0.0 {
            return Err(Error::BehindCamera);
        }

        let intersection_point = camera_position + camera_ray_over_ground * intersection_scalar;

        let elevated_ground_to_ground =
            Isometry3::<ElevatedGround, Ground>::from(vector![0.0, 0.0, -z]);
        let intersection_point_in_ground = elevated_ground_to_ground * intersection_point;

        Ok(intersection_point_in_ground.xy())
    }

    fn ground_to_pixel(&self, ground_coordinates: Point2<Ground>) -> Result<Point2<Pixel>, Error> {
        self.ground_with_z_to_pixel(ground_coordinates, 0.0)
    }

    fn ground_with_z_to_pixel(
        &self,
        ground_coordinates: Point2<Ground>,
        z: f32,
    ) -> Result<Point2<Pixel>, Error> {
        self.camera_to_pixel(
            (self.ground_to_camera * point![ground_coordinates.x(), ground_coordinates.y(), z])
                .coords(),
        )
    }

    fn pixel_to_robot_with_x(
        &self,
        pixel_coordinates: Point2<Pixel>,
        x: f32,
    ) -> Result<Point3<Robot>, Error> {
        if x <= 0.0 {
            return Err(Error::BehindCamera);
        }

        let camera_ray = self.pixel_to_camera(pixel_coordinates);
        let camera_ray_over_ground = self.camera_to_ground * camera_ray;

        let distance_to_plane = x - self.camera_to_ground.translation().x();
        let slope = distance_to_plane / camera_ray_over_ground.x();

        let intersection_point =
            self.camera_to_ground.translation().coords() + camera_ray_over_ground * slope;
        Ok(point![x, intersection_point.y(), intersection_point.z()])
    }

    fn robot_to_pixel(&self, robot_coordinates: Point3<Robot>) -> Result<Point2<Pixel>, Error> {
        let camera_coordinates = self.robot_to_camera * robot_coordinates;
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
            self.ground_to_camera * point![robot_coordinates.x(), robot_coordinates.y(), 0.0];
        let distance = camera_coordinates.coords().norm();
        if distance <= radius_in_robot_coordinates {
            return Err(Error::TooClose);
        }
        let angle = (radius_in_robot_coordinates / distance).asin();
        Ok(resolution.y() as f32 * angle / self.field_of_view.y)
    }
}
