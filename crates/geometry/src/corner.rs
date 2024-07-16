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
    pub fn from_line_and_point_orthogonal(line: Line2<Frame>, point: Point2<Frame>) -> Self {
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
        let difference_to_point = point - self.point;

        let projected_point1 = self.point
            + (self.direction1 * self.direction1.dot(difference_to_point).max(0.0)
                / self.direction1.norm_squared());
        let projected_point2 = self.point
            + (self.direction2 * self.direction2.dot(difference_to_point).max(0.0)
                / self.direction2.norm_squared());

        distance_squared(point, projected_point1).min(distance_squared(point, projected_point2))
    }
}
