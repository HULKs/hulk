use linear_algebra::{distance_squared, Point2, Vector2};
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

use crate::{line::Line2, Distance};

/// A corner given by a point and the directions of two outgoing rays.
#[derive(
    Copy,
    Clone,
    Debug,
    Default,
    Deserialize,
    PartialEq,
    Serialize,
    PathSerialize,
    PathIntrospect,
    PathDeserialize,
)]
pub struct Corner<Frame> {
    pub point: Point2<Frame>,
    pub direction1: Vector2<Frame>,
    pub direction2: Vector2<Frame>,
}

impl<Frame> Corner<Frame> {
    /// Creates an orthogonal corner from a line and a point outside the line.
    pub fn from_line_and_point_orthogonal(line: &Line2<Frame>, point: Point2<Frame>) -> Self {
        let corner_point = line.closest_point(point);
        let direction1 = line.direction;
        let direction2 = point - corner_point;

        Self {
            point: corner_point,
            direction1,
            direction2,
        }
    }
}

impl<Frame> Distance<Point2<Frame>> for Corner<Frame> {
    fn squared_distance_to(&self, point: Point2<Frame>) -> f32 {
        let difference_to = point - self.point;

        let projected_point1 = self.point
            + (self.direction1 * self.direction1.dot(difference_to).max(0.0)
                / self.direction1.norm_squared());
        let projected_point2 = self.point
            + (self.direction2 * self.direction2.dot(difference_to).max(0.0)
                / self.direction2.norm_squared());

        distance_squared(point, projected_point1).min(distance_squared(point, projected_point2))
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;
    use linear_algebra::{distance, point, vector};

    use crate::line::Line;

    use super::*;

    #[derive(Debug, Clone, Copy)]
    struct SomeFrame;

    const CORNER: Corner<SomeFrame> = Corner {
        point: point![5.0, 5.0],
        direction1: vector![0.0, 3.0],
        direction2: vector![-10.0, 0.0],
    };

    #[test]
    fn is_orthogonal() {
        let line: Line2<SomeFrame> = Line {
            point: point![0.0, 0.0],
            direction: vector![10.0, 0.0],
        };
        let point = point![15.0, 5.0];
        let corner_point = point![15.0, 0.0];

        let corner = Corner::from_line_and_point_orthogonal(&line, point);
        assert_relative_eq!(corner.point, corner_point);
        assert_relative_eq!(corner.direction1.dot(corner.direction2), 0.0);
    }

    #[test]
    fn correct_distance_top_left() {
        let point = point![0.0, 15.0];
        let distance = 5.0;
        let squared_distance = distance * distance;

        assert_relative_eq!(CORNER.distance_to(point), distance);
        assert_relative_eq!(CORNER.squared_distance_to(point), squared_distance);
    }

    #[test]
    fn correct_distance_top_right() {
        let point = point![15.0, 15.0];
        let distance = 10.0;
        let squared_distance = distance * distance;

        assert_relative_eq!(CORNER.distance_to(point), distance);
        assert_relative_eq!(CORNER.squared_distance_to(point), squared_distance);
    }

    #[test]
    fn correct_distance_bottom_left() {
        let point = point![0.0, 0.0];
        let distance = 5.0;
        let squared_distance = distance * distance;

        assert_relative_eq!(CORNER.distance_to(point), distance);
        assert_relative_eq!(CORNER.squared_distance_to(point), squared_distance);
    }

    #[test]
    fn correct_distance_bottom_right() {
        let point = point![10.0, -5.0];
        let distance = distance(CORNER.point, point);
        let squared_distance = distance * distance;

        assert_relative_eq!(CORNER.distance_to(point), distance);
        assert_relative_eq!(CORNER.squared_distance_to(point), squared_distance);
    }
}
