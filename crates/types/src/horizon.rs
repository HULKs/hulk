use approx::{AbsDiffEq, RelativeEq};
use nalgebra::{Isometry3, Point2, Vector2};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize, SerializeHierarchy)]
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

    pub fn from_parameters(
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

impl AbsDiffEq for Horizon {
    type Epsilon = f32;

    fn default_epsilon() -> Self::Epsilon {
        Self::Epsilon::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.left_horizon_y
            .abs_diff_eq(&other.left_horizon_y, epsilon)
            && self
                .right_horizon_y
                .abs_diff_eq(&other.right_horizon_y, epsilon)
    }
}

impl RelativeEq for Horizon {
    fn default_max_relative() -> Self::Epsilon {
        Self::Epsilon::default_max_relative()
    }

    fn relative_eq(
        &self,
        other: &Self,
        epsilon: Self::Epsilon,
        max_relative: Self::Epsilon,
    ) -> bool {
        self.left_horizon_y
            .relative_eq(&other.left_horizon_y, epsilon, max_relative)
            && self
                .right_horizon_y
                .relative_eq(&other.right_horizon_y, epsilon, max_relative)
    }
}
