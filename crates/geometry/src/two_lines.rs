use linear_algebra::{Point2, Vector2};
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

use crate::{
    line::{Line, Line2},
    Distance,
};

/// Two intersecting lines given by their intersection point and directions.
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
pub struct TwoLines<Frame> {
    pub intersection_point: Point2<Frame>,
    pub first_direction: Vector2<Frame>,
    pub second_direction: Vector2<Frame>,
}

impl<Frame> TwoLines<Frame> {
    /// Creates two orthogonal lines from a line and a point outside the line.
    pub fn from_line_and_point_orthogonal(line: &Line2<Frame>, point: Point2<Frame>) -> Self {
        let intersection_point = line.closest_point(point);
        let direction1 = line.direction;
        let direction2 = point - intersection_point;

        Self {
            intersection_point,
            first_direction: direction1,
            second_direction: direction2,
        }
    }
}

impl<Frame> Distance<Point2<Frame>> for TwoLines<Frame> {
    fn squared_distance_to(&self, point: Point2<Frame>) -> f32 {
        let first_line = Line {
            point: self.intersection_point,
            direction: self.first_direction,
        };
        let second_line = Line {
            point: self.intersection_point,
            direction: self.second_direction,
        };

        let squared_distance_to_first_line = first_line.squared_distance_to(point);
        let squared_distance_to_second_line = second_line.squared_distance_to(point);

        squared_distance_to_first_line.min(squared_distance_to_second_line)
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;
    use linear_algebra::{point, vector};

    use crate::line::Line;

    use super::*;

    #[derive(Debug, Clone, Copy)]
    struct SomeFrame;

    const TWO_LINES: TwoLines<SomeFrame> = TwoLines {
        intersection_point: point![5.0, 5.0],
        first_direction: vector![0.0, 3.0],
        second_direction: vector![-10.0, 0.0],
    };

    #[test]
    fn is_orthogonal() {
        let line: Line2<SomeFrame> = Line {
            point: point![0.0, 0.0],
            direction: vector![10.0, 0.0],
        };
        let point = point![15.0, 5.0];
        let corner_point = point![15.0, 0.0];

        let corner = TwoLines::from_line_and_point_orthogonal(&line, point);
        assert_relative_eq!(corner.intersection_point, corner_point);
        assert_relative_eq!(corner.first_direction.dot(corner.second_direction), 0.0);
    }

    #[test]
    fn correct_distance_top_left() {
        let point = point![0.0, 15.0];
        let distance = 5.0;
        let squared_distance = distance * distance;

        assert_relative_eq!(TWO_LINES.distance_to(point), distance);
        assert_relative_eq!(TWO_LINES.squared_distance_to(point), squared_distance);
    }

    #[test]
    fn correct_distance_top_right() {
        let point = point![15.0, 15.0];
        let distance = 10.0;
        let squared_distance = distance * distance;

        assert_relative_eq!(TWO_LINES.distance_to(point), distance);
        assert_relative_eq!(TWO_LINES.squared_distance_to(point), squared_distance);
    }

    #[test]
    fn correct_distance_bottom_left() {
        let point = point![0.0, 0.0];
        let distance = 5.0;
        let squared_distance = distance * distance;

        assert_relative_eq!(TWO_LINES.distance_to(point), distance);
        assert_relative_eq!(TWO_LINES.squared_distance_to(point), squared_distance);
    }

    #[test]
    fn correct_distance_bottom_right() {
        let point = point![10.0, -5.0];
        let distance = 5.0;
        let squared_distance = distance * distance;

        assert_relative_eq!(TWO_LINES.distance_to(point), distance);
        assert_relative_eq!(TWO_LINES.squared_distance_to(point), squared_distance);
    }
}
