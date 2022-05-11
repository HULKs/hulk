use approx::{AbsDiffEq, RelativeEq};
use macros::SerializeHierarchy;
use nalgebra::{vector, Point2};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize, SerializeHierarchy)]
pub struct Circle {
    pub center: Point2<f32>,
    pub radius: f32,
}

impl AbsDiffEq for Circle {
    type Epsilon = f32;

    fn default_epsilon() -> Self::Epsilon {
        Self::Epsilon::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.center.abs_diff_eq(&other.center, epsilon)
            && self.radius.abs_diff_eq(&other.radius, epsilon)
    }
}

impl RelativeEq for Circle {
    fn default_max_relative() -> Self::Epsilon {
        Self::Epsilon::default_max_relative()
    }

    fn relative_eq(
        &self,
        other: &Self,
        epsilon: Self::Epsilon,
        max_relative: Self::Epsilon,
    ) -> bool {
        self.center
            .relative_eq(&other.center, epsilon, max_relative)
            && self
                .radius
                .relative_eq(&other.radius, epsilon, max_relative)
    }
}

impl Circle {
    pub fn bounding_box(&self) -> Rectangle {
        let radius_vector = vector![self.radius, self.radius];

        Rectangle {
            top_left: self.center - radius_vector,
            bottom_right: self.center + radius_vector,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize, SerializeHierarchy)]
pub struct Rectangle {
    pub top_left: Point2<f32>,
    pub bottom_right: Point2<f32>,
}

impl Rectangle {
    pub fn rectangle_intersection(self, other: Rectangle) -> f32 {
        let intersection_x = f32::max(
            0.0,
            f32::min(self.bottom_right.x, other.bottom_right.x)
                - f32::max(self.top_left.x, other.top_left.x),
        );
        let intersection_y = f32::max(
            0.0,
            f32::min(self.bottom_right.y, other.bottom_right.y)
                - f32::max(self.top_left.y, other.top_left.y),
        );
        intersection_x * intersection_y
    }

    pub fn area(self) -> f32 {
        let dimensions = self.bottom_right - self.top_left;
        dimensions.x * dimensions.y
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::PI;

    use approx::{assert_relative_eq, assert_relative_ne};
    use nalgebra::point;

    use super::*;

    #[test]
    fn circle_cmp_same() {
        assert_relative_eq!(
            Circle {
                center: point![1337.5, 42.5],
                radius: f32::sqrt(2.0),
            },
            Circle {
                center: point![1337.5, 42.5],
                radius: f32::sin(PI / 4.0) * 2.0,
            },
        );
    }

    #[test]
    fn circle_cmp_different_radius() {
        assert_relative_ne!(
            Circle {
                center: point![1337.5, 42.5],
                radius: f32::sqrt(3.0),
            },
            Circle {
                center: point![1337.5, 42.5],
                radius: f32::sin(PI / 4.0) * 2.0,
            },
        );
    }

    #[test]
    fn circle_cmp_different_center() {
        assert_relative_ne!(
            Circle {
                center: point![1337.1, 42.5],
                radius: f32::sqrt(2.0),
            },
            Circle {
                center: point![1337.5, 52.5],
                radius: f32::sin(PI / 4.0) * 2.0,
            },
        );
    }
}
