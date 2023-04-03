use nalgebra::{Isometry3, Matrix, Point2, Vector2};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

use crate::horizon::Horizon;

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
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;
    use nalgebra::vector;

    use super::*;

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
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct ProjectedFieldLines {
    pub top: Vec<Line2>,
    pub bottom: Vec<Line2>,
}
