use approx::relative_eq;
use coordinate_systems::{Framed, IntoFramed};
use nalgebra::{point, vector, Isometry3, Point2, Point3, Vector2, Vector3};
use thiserror::Error;
use types::{
    camera_matrix::CameraMatrix,
    coordinate_systems::{Camera, Ground, Pixel, Robot},
};

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
    fn pixel_to_camera(
        &self,
        pixel_coordinates: Framed<Pixel, Point2<f32>>,
    ) -> Framed<Camera, Vector3<f32>>;
    fn camera_to_pixel(
        &self,
        camera_ray: Framed<Camera, Vector3<f32>>,
    ) -> Result<Framed<Pixel, Point2<f32>>, Error>;
    fn pixel_to_ground(
        &self,
        pixel_coordinates: Framed<Pixel, Point2<f32>>,
    ) -> Result<Framed<Ground, Point2<f32>>, Error>;
    fn pixel_to_ground_with_z(
        &self,
        pixel_coordinates: Framed<Pixel, Point2<f32>>,
        z: f32,
    ) -> Result<Framed<Ground, Point2<f32>>, Error>;
    fn ground_to_pixel(
        &self,
        ground_coordinates: Framed<Ground, Point2<f32>>,
    ) -> Result<Framed<Pixel, Point2<f32>>, Error>;
    fn ground_with_z_to_pixel(
        &self,
        ground_coordinates: Framed<Ground, Point2<f32>>,
        z: f32,
    ) -> Result<Framed<Pixel, Point2<f32>>, Error>;
    fn pixel_to_robot_with_x(
        &self,
        pixel_coordinates: Framed<Pixel, Point2<f32>>,
        x: f32,
    ) -> Result<Framed<Robot, Point3<f32>>, Error>;
    fn robot_to_pixel(
        &self,
        robot_coordinates: Framed<Robot, Point3<f32>>,
    ) -> Result<Framed<Pixel, Point2<f32>>, Error>;
    fn get_pixel_radius(
        &self,
        radius_in_robot_coordinates: f32,
        pixel_coordinates: Framed<Pixel, Point2<f32>>,
        resolution: Vector2<u32>,
    ) -> Result<f32, Error>;
}

impl Projection for CameraMatrix {
    fn pixel_to_camera(
        &self,
        pixel_coordinates: Framed<Pixel, Point2<f32>>,
    ) -> Framed<Camera, Vector3<f32>> {
        vector![
            1.0,
            (self.optical_center.x - pixel_coordinates.inner.x) / self.focal_length.x,
            (self.optical_center.y - pixel_coordinates.inner.y) / self.focal_length.y,
        ]
        .framed()
    }

    fn camera_to_pixel(
        &self,
        camera_ray: Framed<Camera, Vector3<f32>>,
    ) -> Result<Framed<Pixel, Point2<f32>>, Error> {
        if camera_ray.inner.x <= 0.0 {
            return Err(Error::BehindCamera);
        }
        Ok(point![
            self.optical_center.x - self.focal_length.x * camera_ray.inner.y / camera_ray.inner.x,
            self.optical_center.y - self.focal_length.y * camera_ray.inner.z / camera_ray.inner.x,
        ]
        .framed())
    }

    fn pixel_to_ground(
        &self,
        pixel_coordinates: Framed<Pixel, Point2<f32>>,
    ) -> Result<Framed<Ground, Point2<f32>>, Error> {
        self.pixel_to_ground_with_z(pixel_coordinates, 0.0)
    }

    fn pixel_to_ground_with_z(
        &self,
        pixel_coordinates: Framed<Pixel, Point2<f32>>,
        z: f32,
    ) -> Result<Framed<Ground, Point2<f32>>, Error> {
        let camera_ray = self.pixel_to_camera(pixel_coordinates);
        let camera_to_elevated_ground = Isometry3::translation(0., 0., -z) * self.camera_to_ground.inner;

        let camera_position = camera_to_elevated_ground * Point3::origin();
        let camera_ray_over_ground = camera_to_elevated_ground * camera_ray.inner;

        if relative_eq!(camera_ray_over_ground.z, 0.0) {
            return Err(Error::NotOnProjectionPlane);
        }

        let intersection_scalar = -camera_position.z / camera_ray_over_ground.z;

        if intersection_scalar < 0.0 {
            return Err(Error::BehindCamera);
        }

        let intersection_point = camera_position + camera_ray_over_ground * intersection_scalar;

        Ok(intersection_point.xy().framed())
    }

    fn ground_to_pixel(
        &self,
        ground_coordinates: Framed<Ground, Point2<f32>>,
    ) -> Result<Framed<Pixel, Point2<f32>>, Error> {
        self.ground_with_z_to_pixel(ground_coordinates, 0.0)
    }

    fn ground_with_z_to_pixel(
        &self,
        ground_coordinates: Framed<Ground, Point2<f32>>,
        z: f32,
    ) -> Result<Framed<Pixel, Point2<f32>>, Error> {
        self.camera_to_pixel(
            (self.ground_to_camera
                * point![ground_coordinates.inner.x, ground_coordinates.inner.y, z].framed())
            .coords(),
        )
    }

    fn pixel_to_robot_with_x(
        &self,
        pixel_coordinates: Framed<Pixel, Point2<f32>>,
        x: f32,
    ) -> Result<Framed<Robot, Point3<f32>>, Error> {
        if x <= 0.0 {
            return Err(Error::BehindCamera);
        }

        let camera_ray = self.pixel_to_camera(pixel_coordinates);
        let camera_ray_over_robot = self.camera_to_ground.inner.rotation * camera_ray.inner;

        let distance_to_plane = x - self.camera_to_ground.inner.translation.x;
        let slope = distance_to_plane / camera_ray_over_robot.x;

        let intersection_point =
            self.camera_to_ground.inner.translation.vector + camera_ray_over_robot * slope;
        Ok(point![x, intersection_point.y, intersection_point.z].framed())
    }

    fn robot_to_pixel(
        &self,
        robot_coordinates: Framed<Robot, Point3<f32>>,
    ) -> Result<Framed<Pixel, Point2<f32>>, Error> {
        let camera_coordinates = self.robot_to_camera * robot_coordinates;
        self.camera_to_pixel(camera_coordinates.inner.coords.framed())
    }

    fn get_pixel_radius(
        &self,
        radius_in_robot_coordinates: f32,
        pixel_coordinates: Framed<Pixel, Point2<f32>>,
        resolution: Vector2<u32>,
    ) -> Result<f32, Error> {
        let robot_coordinates =
            self.pixel_to_ground_with_z(pixel_coordinates, radius_in_robot_coordinates)?;
        let camera_coordinates = self.ground_to_camera
            * point![robot_coordinates.inner.x, robot_coordinates.inner.y, 0.0].framed();
        let distance = camera_coordinates.inner.coords.norm();
        if distance <= radius_in_robot_coordinates {
            return Err(Error::TooClose);
        }
        let angle = (radius_in_robot_coordinates / distance).asin();
        Ok(resolution.y as f32 * angle / self.field_of_view.y)
    }
}
