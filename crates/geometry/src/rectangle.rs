use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

use linear_algebra::{point, Point2, Vector2};

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
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

    pub fn project_point_into_rect(&self, point: Point2<Frame>) -> Point2<Frame> {
        point![
            point.x().clamp(self.min.x(), self.max.x()),
            point.y().clamp(self.min.y(), self.max.y()),
        ]
    }

    pub fn center(&self) -> Point2<Frame> {
        point![
            (self.min.x() + self.max.x()) / 2.0,
            (self.min.y() + self.max.y()) / 2.0
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linear_algebra::point;

    // Test coordinate system
    #[derive(Debug)]
    struct S;

    #[test]
    fn test_rectangle_intersection() {
        let rect1 = Rectangle::<S> {
            min: point![0.0, 0.0],
            max: point![2.0, 2.0],
        };
        let rect2 = Rectangle::<S> {
            min: point![1.0, 1.0],
            max: point![3.0, 3.0],
        };
        assert_eq!(rect1.rectangle_intersection(rect2), 1.0);
    }

    #[test]
    fn test_area() {
        let rect = Rectangle::<S> {
            min: point![0.0, 0.0],
            max: point![2.0, 3.0],
        };
        assert_eq!(rect.area(), 6.0);
    }

    #[test]
    fn test_project_point_in_rect() {
        let rect = Rectangle::<S> {
            min: point![0.0, 0.0],
            max: point![2.0, 2.0],
        };
        let point_inside = point![1.0, 1.0];
        assert_eq!(rect.project_point_into_rect(point_inside), point_inside);
    }

    #[test]
    fn test_project_point_outside_rect() {
        let rect = Rectangle::<S> {
            min: point![0.0, 0.0],
            max: point![2.0, 2.0],
        };
        let point_outside = point![3.0, -1.0];
        assert_eq!(
            rect.project_point_into_rect(point_outside),
            point![2.0, 0.0]
        );
    }
}
