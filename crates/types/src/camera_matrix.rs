use color_eyre::{eyre::bail, Result};
use nalgebra::{point, vector, Isometry3, Matrix, Point2, Point3, Vector2, Vector3};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

use super::Line2;

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct CameraMatrices {
    pub top: CameraMatrix,
    pub bottom: CameraMatrix,
}

#[derive(Clone, Debug, Deserialize, Serialize, SerializeHierarchy)]
pub struct CameraMatrix {
    pub camera_to_head: Isometry3<f32>,
    pub camera_to_ground: Isometry3<f32>,
    pub ground_to_camera: Isometry3<f32>,
    pub camera_to_robot: Isometry3<f32>,
    pub robot_to_camera: Isometry3<f32>,
    pub focal_length: Vector2<f32>,
    pub optical_center: Point2<f32>,
    pub field_of_view: Vector2<f32>,
    pub horizon: Horizon,
}

impl Default for CameraMatrix {
    fn default() -> Self {
        Self {
            camera_to_head: Isometry3::identity(),
            camera_to_ground: Isometry3::identity(),
            ground_to_camera: Isometry3::identity(),
            camera_to_robot: Isometry3::identity(),
            robot_to_camera: Isometry3::identity(),
            focal_length: Default::default(),
            optical_center: Point2::origin(),
            field_of_view: Default::default(),
            horizon: Default::default(),
        }
    }
}

impl CameraMatrix {
    /// This takes [0, 1] range focal length & optical center values & actual image size to create camera matrix.
    pub fn from_normalized_focal_and_center(
        focal_length: Vector2<f32>,
        optical_center: Point2<f32>,
        image_size: Vector2<f32>,
        camera_to_head: Isometry3<f32>,
        head_to_robot: Isometry3<f32>,
        robot_to_ground: Isometry3<f32>,
    ) -> Self {
        let camera_to_robot = head_to_robot * camera_to_head;
        let camera_to_ground = robot_to_ground * camera_to_robot;

        let image_size_diagonal = Matrix::from_diagonal(&image_size);
        let focal_length_scaled = image_size_diagonal * focal_length;
        let optical_center_scaled = image_size_diagonal * optical_center;

        let field_of_view = CameraMatrix::calculate_field_of_view(focal_length_scaled, image_size);

        let horizon = Horizon::from_parameters(
            camera_to_ground,
            focal_length_scaled,
            optical_center_scaled,
            image_size[0],
        );

        Self {
            camera_to_head,
            camera_to_ground,
            ground_to_camera: camera_to_ground.inverse(),
            camera_to_robot,
            robot_to_camera: camera_to_robot.inverse(),
            focal_length: focal_length_scaled,
            optical_center: optical_center_scaled,
            field_of_view,
            horizon,
        }
    }

    fn calculate_field_of_view(
        focal_lengths: Vector2<f32>,
        image_size: Vector2<f32>,
    ) -> Vector2<f32> {
        // Ref:  https://www.edmundoptics.eu/knowledge-center/application-notes/imaging/understanding-focal-length-and-field-of-view/
        image_size.zip_map(&focal_lengths, |image_dim, focal_length| -> f32 {
            2.0 * (image_dim * 0.5 / focal_length).atan()
        })
    }

    pub fn pixel_to_camera(&self, pixel_coordinates: &Point2<f32>) -> Vector3<f32> {
        vector![
            1.0,
            (self.optical_center.x - pixel_coordinates.x) / self.focal_length.x,
            (self.optical_center.y - pixel_coordinates.y) / self.focal_length.y
        ]
    }

    pub fn camera_to_pixel(&self, camera_ray: &Vector3<f32>) -> Result<Point2<f32>> {
        if camera_ray.x <= 0.0 {
            bail!("ray points behind the camera")
        }
        Ok(point![
            self.optical_center.x - self.focal_length.x * camera_ray.y / camera_ray.x,
            self.optical_center.y - self.focal_length.y * camera_ray.z / camera_ray.x
        ])
    }

    pub fn pixel_to_ground(&self, pixel_coordinates: &Point2<f32>) -> Result<Point2<f32>> {
        self.pixel_to_ground_with_z(pixel_coordinates, 0.0)
    }

    pub fn pixel_to_ground_with_z(
        &self,
        pixel_coordinates: &Point2<f32>,
        z: f32,
    ) -> Result<Point2<f32>> {
        let camera_ray = self.pixel_to_camera(pixel_coordinates);
        let camera_ray_rotated_to_robot_coordinate_system =
            self.camera_to_ground.rotation * camera_ray;
        if camera_ray_rotated_to_robot_coordinate_system.z >= 0.0
            || camera_ray_rotated_to_robot_coordinate_system.x.is_nan()
            || camera_ray_rotated_to_robot_coordinate_system.y.is_nan()
            || camera_ray_rotated_to_robot_coordinate_system.z.is_nan()
        {
            bail!("cannot map pixel to ground because it is above the horizon");
        }

        Ok(point![
            self.camera_to_ground.translation.x
                - (self.camera_to_ground.translation.z - z)
                    * camera_ray_rotated_to_robot_coordinate_system.x
                    / camera_ray_rotated_to_robot_coordinate_system.z,
            self.camera_to_ground.translation.y
                - (self.camera_to_ground.translation.z - z)
                    * camera_ray_rotated_to_robot_coordinate_system.y
                    / camera_ray_rotated_to_robot_coordinate_system.z
        ])
    }

    pub fn ground_to_pixel(&self, ground_coordinates: &Point2<f32>) -> Result<Point2<f32>> {
        self.ground_with_z_to_pixel(ground_coordinates, 0.0)
    }

    pub fn ground_with_z_to_pixel(
        &self,
        ground_coordinates: &Point2<f32>,
        z: f32,
    ) -> Result<Point2<f32>> {
        self.camera_to_pixel(
            &(self.ground_to_camera * point![ground_coordinates.x, ground_coordinates.y, z]).coords,
        )
    }

    pub fn robot_to_pixel(&self, robot_coordinates: &Point3<f32>) -> Result<Point2<f32>> {
        let camera_coordinates = self.robot_to_camera * robot_coordinates;
        self.camera_to_pixel(&camera_coordinates.coords)
    }

    pub fn get_pixel_radius(
        &self,
        radius_in_robot_coordinates: f32,
        pixel_coordinates: &Point2<f32>,
        resolution: &Vector2<u32>,
    ) -> Result<f32> {
        let robot_coordinates =
            self.pixel_to_ground_with_z(pixel_coordinates, radius_in_robot_coordinates)?;
        let camera_coordinates =
            self.ground_to_camera * point![robot_coordinates.x, robot_coordinates.y, 0.0];
        let distance = camera_coordinates.coords.norm();
        if distance <= radius_in_robot_coordinates {
            bail!("object too close to camera to calculate pixel radius");
        }
        let angle = (radius_in_robot_coordinates / distance).asin();
        Ok(resolution.y as f32 * angle / self.field_of_view.y)
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct Horizon {
    pub left_horizon_y: f32,
    pub right_horizon_y: f32,
}

impl Horizon {
    pub fn horizon_y_minimum(&self) -> f32 {
        self.left_horizon_y.min(self.right_horizon_y)
    }

    pub fn y_at_x(&self, x: f32, image_width: f32) -> f32 {
        self.left_horizon_y + x / image_width * (self.right_horizon_y - self.left_horizon_y)
    }

    fn from_parameters(
        camera_to_ground: Isometry3<f32>,
        focal_length: Vector2<f32>,
        optical_center: Point2<f32>,
        image_width: f32,
    ) -> Self {
        let rotation_matrix = camera_to_ground.rotation.to_rotation_matrix();
        let horizon_slope_is_infinite = rotation_matrix[(2, 2)] == 0.0;

        if horizon_slope_is_infinite {
            Self {
                left_horizon_y: 0.0,
                right_horizon_y: 0.0,
            }
        } else {
            let left_horizon_y = optical_center.y
                + focal_length.y
                    * (rotation_matrix[(2, 0)]
                        + optical_center.x * rotation_matrix[(2, 1)] / focal_length.x)
                    / rotation_matrix[(2, 2)];
            let slope = -focal_length.y * rotation_matrix[(2, 1)]
                / (focal_length.x * rotation_matrix[(2, 2)]);

            // Guesses if image size is in "normalized" (1.0 x 1.0) dimensions
            let adjusted_image_width = if image_width <= 1.0 {
                image_width
            } else {
                image_width - 1.0
            };
            let right_horizon_y = left_horizon_y + (slope * adjusted_image_width);

            Self {
                left_horizon_y,
                right_horizon_y,
            }
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct ProjectedFieldLines {
    pub top: Vec<Line2>,
    pub bottom: Vec<Line2>,
}

#[cfg(test)]
impl CameraMatrix {
    pub fn from_normalized_focal_and_center_short(
        focal_length: Vector2<f32>,
        optical_center: Point2<f32>,
        image_size: Vector2<f32>,
    ) -> CameraMatrix {
        CameraMatrix::from_normalized_focal_and_center(
            focal_length,
            optical_center,
            image_size,
            Isometry3::identity(),
            Isometry3::identity(),
            Isometry3::identity(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use approx::assert_relative_eq;
    use nalgebra::{Translation, UnitQuaternion};

    #[test]
    fn check_field_of_view_calculation() {
        // Old implementation, assumes normalized values
        fn old_fov(focal_lengths: Vector2<f32>) -> Vector2<f32> {
            focal_lengths.map(|f| 2.0 * (0.5 / f).atan())
        }

        let focals = vector![0.63, 1.34];
        let image_size = vector![1.0, 1.0];

        let image_size_abs = vector![640.0, 480.0];
        let focals_scaled = image_size_abs.zip_map(&focals, |dim, focal| dim * focal);

        assert_relative_eq!(
            old_fov(focals),
            CameraMatrix::calculate_field_of_view(focals, image_size)
        );

        assert_relative_eq!(
            old_fov(focals),
            CameraMatrix::calculate_field_of_view(focals_scaled, image_size_abs)
        );
    }

    #[test]
    fn pixel_to_camera_default_center() {
        let camera_matrix = CameraMatrix::from_normalized_focal_and_center_short(
            vector![2.0, 2.0],
            point![1.0, 1.0],
            vector![1.0, 1.0],
        );

        assert_relative_eq!(
            camera_matrix.pixel_to_camera(&point![1.0, 1.0]),
            vector![1.0, 0.0, 0.0]
        );
    }

    #[test]
    fn pixel_to_camera_default_top_left() {
        let camera_matrix = CameraMatrix::from_normalized_focal_and_center_short(
            vector![2.0, 2.0],
            point![1.0, 1.0],
            vector![2.0, 2.0],
        );

        assert_relative_eq!(
            camera_matrix.pixel_to_camera(&point![0.0, 0.0]),
            vector![1.0, 0.5, 0.5]
        );
    }

    #[test]
    fn pixel_to_camera_sample_camera_center() {
        let camera_matrix = CameraMatrix::from_normalized_focal_and_center_short(
            vector![0.95, 1.27],
            point![0.5, 0.5],
            vector![640.0, 480.0],
        );

        assert_relative_eq!(
            camera_matrix.pixel_to_camera(&point![320.0, 240.0]),
            vector![1.0, 0.0, 0.0]
        );
    }

    #[test]
    fn pixel_to_camera_sample_camera_top_left() {
        let camera_matrix = CameraMatrix::from_normalized_focal_and_center_short(
            vector![0.95, 1.27],
            point![0.5, 0.5],
            vector![640.0, 480.0],
        );

        assert_relative_eq!(
            camera_matrix.pixel_to_camera(&point![0.0, 0.0]),
            vector![1.0, 0.5 / 0.95, 0.5 / 1.27]
        );
    }

    #[test]
    fn camera_to_pixel_default_center() -> Result<()> {
        let camera_matrix = CameraMatrix::from_normalized_focal_and_center_short(
            vector![2.0, 2.0],
            point![1.0, 1.0],
            vector![1.0, 1.0],
        );

        assert_relative_eq!(
            camera_matrix.camera_to_pixel(&vector![1.0, 0.0, 0.0])?,
            point![1.0, 1.0]
        );

        Ok(())
    }

    #[test]
    fn camera_to_pixel_default_top_left() -> Result<()> {
        let camera_matrix = CameraMatrix::from_normalized_focal_and_center_short(
            vector![2.0, 2.0],
            point![1.0, 1.0],
            vector![1.0, 1.0],
        );

        assert_relative_eq!(
            camera_matrix.camera_to_pixel(&vector![1.0, 0.5, 0.5])?,
            point![0.0, 0.0]
        );

        Ok(())
    }

    #[test]
    fn camera_to_pixel_sample_camera_center() -> Result<()> {
        let camera_matrix = CameraMatrix::from_normalized_focal_and_center_short(
            vector![0.95, 1.27],
            point![0.5, 0.5],
            vector![640.0, 480.0],
        );

        assert_relative_eq!(
            camera_matrix.camera_to_pixel(&vector![1.0, 0.0, 0.0])?,
            point![320.0, 240.0]
        );

        Ok(())
    }

    #[test]
    fn camera_to_pixel_sample_camera_top_left() -> Result<()> {
        let camera_matrix = CameraMatrix::from_normalized_focal_and_center_short(
            vector![0.95, 1.27],
            point![0.5, 0.5],
            vector![640.0, 480.0],
        );

        assert_relative_eq!(
            camera_matrix.camera_to_pixel(&vector![1.0, 0.5 / 0.95, 0.5 / 1.27])?,
            point![0.0, 0.0],
            epsilon = 0.0001
        );

        Ok(())
    }

    #[test]
    fn pixel_to_camera_reversible() -> Result<()> {
        let camera_matrix = CameraMatrix::from_normalized_focal_and_center_short(
            vector![0.95, 1.27],
            point![0.5, 0.5],
            vector![640.0, 480.0],
        );

        let input = point![512.0, 257.0];
        let output = camera_matrix.camera_to_pixel(&camera_matrix.pixel_to_camera(&input))?;

        assert_relative_eq!(input, output);

        Ok(())
    }

    #[test]
    fn pixel_to_ground_with_z_only_elevation() -> Result<()> {
        let mut camera_matrix = CameraMatrix::from_normalized_focal_and_center_short(
            vector![2.0, 2.0],
            point![1.0, 1.0],
            vector![1.0, 1.0],
        );
        camera_matrix.camera_to_ground.translation = Translation::from(point![0.0, 0.0, 0.5]);

        assert_relative_eq!(
            camera_matrix.pixel_to_ground_with_z(&point![1.0, 2.0], 0.25)?,
            point![0.5, 0.0]
        );
        Ok(())
    }

    #[test]
    fn pixel_to_ground_with_z_pitch_45_degree_down() -> Result<()> {
        let mut camera_matrix = CameraMatrix::from_normalized_focal_and_center_short(
            vector![2.0, 2.0],
            point![1.0, 1.0],
            vector![1.0, 1.0],
        );
        camera_matrix.camera_to_ground.translation = Translation::from(point![0.0, 0.0, 0.5]);
        camera_matrix.camera_to_ground.rotation =
            UnitQuaternion::from_euler_angles(0.0, std::f32::consts::PI / 4.0, 0.0);

        assert_relative_eq!(
            camera_matrix.pixel_to_ground_with_z(&point![1.0, 1.0], 0.0)?,
            point![0.5, 0.0]
        );
        Ok(())
    }

    #[test]
    fn ground_to_pixel_only_elevation() -> Result<()> {
        let mut camera_matrix = CameraMatrix::from_normalized_focal_and_center_short(
            vector![2.0, 2.0],
            point![1.0, 1.0],
            vector![1.0, 1.0],
        );
        camera_matrix.camera_to_ground.translation = Translation::from(point![0.0, 0.0, 0.75]);
        camera_matrix.ground_to_camera = camera_matrix.camera_to_ground.inverse();

        assert_relative_eq!(
            camera_matrix.ground_with_z_to_pixel(&point![1.0, 0.0], 0.25)?,
            point![1.0, 2.0]
        );
        Ok(())
    }

    #[test]
    fn ground_to_pixel_pitch_45_degree_down() -> Result<()> {
        let mut camera_matrix = CameraMatrix::from_normalized_focal_and_center_short(
            vector![1.0, 1.0],
            point![0.5, 0.5],
            vector![1.0, 1.0],
        );
        camera_matrix.camera_to_ground.translation = Translation::from(point![0.0, 0.0, 1.0]);
        camera_matrix.camera_to_ground.rotation =
            UnitQuaternion::from_euler_angles(0.0, std::f32::consts::PI / 4.0, 0.0);
        camera_matrix.ground_to_camera = camera_matrix.camera_to_ground.inverse();

        assert_relative_eq!(
            camera_matrix.ground_with_z_to_pixel(&point![0.5, 0.0], 0.5)?,
            point![0.5, 0.5]
        );
        Ok(())
    }

    #[test]
    fn robot_to_pixel_only_elevation() -> Result<()> {
        let mut camera_matrix = CameraMatrix::from_normalized_focal_and_center_short(
            vector![2.0, 2.0],
            point![1.0, 1.0],
            vector![1.0, 1.0],
        );
        camera_matrix.camera_to_robot.translation = Translation::from(point![0.0, 0.0, 0.75]);
        camera_matrix.robot_to_camera = camera_matrix.camera_to_robot.inverse();

        assert_relative_eq!(
            camera_matrix.robot_to_pixel(&point![1.0, 0.0, 0.25])?,
            point![1.0, 2.0]
        );
        Ok(())
    }

    #[test]
    fn robot_to_pixel_pitch_45_degree_down() -> Result<()> {
        let mut camera_matrix = CameraMatrix::from_normalized_focal_and_center_short(
            vector![1.0, 1.0],
            point![0.5, 0.5],
            vector![1.0, 1.0],
        );
        camera_matrix.camera_to_robot.translation = Translation::from(point![0.0, 0.0, 1.0]);
        camera_matrix.camera_to_robot.rotation =
            UnitQuaternion::from_euler_angles(0.0, std::f32::consts::PI / 4.0, 0.0);
        camera_matrix.robot_to_camera = camera_matrix.camera_to_robot.inverse();

        assert_relative_eq!(
            camera_matrix.robot_to_pixel(&point![0.5, 0.0, 0.5])?,
            point![0.5, 0.5]
        );
        Ok(())
    }

    #[test]
    fn get_pixel_radius_only_elevation() -> Result<()> {
        let mut camera_matrix = CameraMatrix::from_normalized_focal_and_center_short(
            vector![1.0, 1.0],
            point![0.5, 0.5],
            vector![640.0, 480.0],
        );
        camera_matrix.field_of_view = vector![45.0, 45.0].map(|a: f32| a.to_radians());
        camera_matrix.camera_to_ground.translation = Translation::from(point![0.0, 0.0, 0.5]);

        assert_relative_eq!(
            camera_matrix.get_pixel_radius(0.05, &point![320.0, 480.0], &vector![640, 480])?,
            33.970547
        );
        Ok(())
    }

    #[test]
    fn get_pixel_radius_pitch_45_degree_down() -> Result<()> {
        let mut camera_matrix = CameraMatrix::from_normalized_focal_and_center_short(
            vector![1.0, 1.0],
            point![0.5, 0.5],
            vector![640.0, 480.0],
        );
        camera_matrix.field_of_view = vector![45.0, 45.0].map(|a: f32| a.to_radians());
        camera_matrix.camera_to_ground.translation = Translation::from(point![0.0, 0.0, 0.5]);
        camera_matrix.camera_to_ground.rotation =
            UnitQuaternion::from_euler_angles(0.0, std::f32::consts::PI / 4.0, 0.0);

        assert_relative_eq!(
            camera_matrix.get_pixel_radius(0.05, &point![320.0, 480.0], &vector![640, 480])?,
            207.69307
        );
        Ok(())
    }
}
