use coordinate_systems::{Camera, Ground, Head, Pixel, Robot};
use linear_algebra::{IntoFramed, Isometry3, Point2, Vector2};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

use crate::{horizon::Horizon, intrinsic::Intrinsic};


#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize, SerializeHierarchy)]
#[serialize_hierarchy(
    bound = "Camera: SerializeHierarchy + Serialize, for<'de> Camera: Deserialize<'de>"
)]
pub struct CameraMatrix {
    pub ground_to_robot: Isometry3<Ground, Robot>,
    pub robot_to_head: Isometry3<Robot, Head>,
    pub head_to_camera: Isometry3<Head, Camera>,
    pub intrinsics: Intrinsic,
    pub focal_length: nalgebra::Vector2<f32>,
    pub optical_center: Point2<Pixel>,
    pub field_of_view: nalgebra::Vector2<f32>,
    pub horizon: Horizon,
    pub image_size: Vector2<Pixel>,

    // Precomputed values for faster calculations
    // pub ground_to_camera: Isometry3<Ground, Camera>,
    // pub ground_to_pixel: 

}

impl CameraMatrix {
    /// This takes [0, 1] range focal length & optical center values & actual image size to create camera matrix.
    pub fn from_normalized_focal_and_center(
        focal_length: nalgebra::Vector2<f32>,
        optical_center: nalgebra::Point2<f32>,
        image_size: Vector2<Pixel>,
        ground_to_robot: Isometry3<Ground, Robot>,
        robot_to_head: Isometry3<Robot, Head>,
        head_to_camera: Isometry3<Head, Camera>,
    ) -> Self {
        let focal_length_scaled = focal_length.component_mul(&image_size.inner);
        let optical_center_scaled = optical_center
            .coords
            .component_mul(&image_size.inner)
            .framed()
            .as_point();

        let field_of_view = Self::calculate_field_of_view(focal_length_scaled, image_size);

        let ground_to_camera = head_to_camera * robot_to_head * ground_to_robot;

        let intrinsics = Intrinsic::new(focal_length_scaled, optical_center_scaled);

        let horizon = Horizon::from_parameters(ground_to_camera, &intrinsics);

        Self {
            intrinsics,
            focal_length: focal_length_scaled,
            optical_center: optical_center_scaled,
            field_of_view,
            horizon,
            ground_to_robot,
            robot_to_head,
            head_to_camera,
            image_size,
        }
    }

    pub fn calculate_field_of_view(
        focal_lengths: nalgebra::Vector2<f32>,
        image_size: Vector2<Pixel>,
    ) -> nalgebra::Vector2<f32> {
        // Ref:  https://www.edmundoptics.eu/knowledge-center/application-notes/imaging/understanding-focal-length-and-field-of-view/
        image_size
            .inner
            .zip_map(&focal_lengths, |image_dim, focal_length| -> f32 {
                2.0 * (image_dim * 0.5 / focal_length).atan()
            })
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;
    use linear_algebra::vector;

    use super::*;

    #[test]
    fn check_field_of_view_calculation() {
        // Old implementation, assumes normalized values
        fn old_fov(focal_lengths: nalgebra::Vector2<f32>) -> nalgebra::Vector2<f32> {
            focal_lengths.map(|f| 2.0 * (0.5 / f).atan())
        }

        let focals = nalgebra::vector![0.63, 1.34];
        let image_size = vector![1.0, 1.0];
        let image_size_abs = vector![640.0, 480.0];

        let focals_scaled = image_size_abs
            .inner
            .zip_map(&focals, |dim, focal| dim as f32 * focal);

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