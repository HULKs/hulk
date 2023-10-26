use nalgebra::{Point2, Vector2};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize, SerializeHierarchy)]
pub struct Rectangle {
    pub min: Point2<f32>,
    pub max: Point2<f32>,
}

impl Rectangle {
    pub fn new_with_center_and_size(center: Point2<f32>, size: Vector2<f32>) -> Self {
        Self {
            min: center - size / 2.0,
            max: center + size / 2.0,
        }
    }
    pub fn rectangle_intersection(self, other: Rectangle) -> f32 {
        let intersection_x = f32::max(
            0.0,
            f32::min(self.max.x, other.max.x) - f32::max(self.min.x, other.min.x),
        );
        let intersection_y = f32::max(
            0.0,
            f32::min(self.max.y, other.max.y) - f32::max(self.min.y, other.min.y),
        );
        intersection_x * intersection_y
    }

    pub fn area(self) -> f32 {
        let dimensions = self.max - self.min;
        dimensions.x * dimensions.y
    }
}
