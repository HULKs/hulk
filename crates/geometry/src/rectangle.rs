use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

use linear_algebra::{Point2, Vector2};

#[derive(
    Clone,
    Copy,
    Debug,
    Deserialize,
    PartialEq,
    PathDeserialize,
    PathIntrospect,
    PathSerialize,
    Serialize,
)]
pub struct Rectangle<Frame> {
    pub min: Point2<Frame>,
    pub max: Point2<Frame>,
}

impl<Frame> Rectangle<Frame> {
    pub fn new_with_center_and_size(center: Point2<Frame>, size: Vector2<Frame>) -> Self {
        Self {
            min: center - size / 2.0,
            max: center + size / 2.0,
        }
    }
    pub fn rectangle_intersection(self, other: Rectangle<Frame>) -> f32 {
        let intersection_x = f32::max(
            0.0,
            f32::min(self.max.x(), other.max.x()) - f32::max(self.min.x(), other.min.x()),
        );
        let intersection_y = f32::max(
            0.0,
            f32::min(self.max.y(), other.max.y()) - f32::max(self.min.y(), other.min.y()),
        );
        intersection_x * intersection_y
    }

    pub fn area(self) -> f32 {
        let dimensions = self.max - self.min;
        dimensions.x() * dimensions.y()
    }

    pub fn contains(self, point: Point2<Frame>) -> bool {
        (self.min.x()..=self.max.x()).contains(&point.x())
            && (self.min.y()..=self.max.y()).contains(&point.y())
    }
}
