use std::ops::Mul;

use approx::{AbsDiffEq, RelativeEq};
use serde::{Deserialize, Serialize};

use linear_algebra::{distance_squared, Point, Point2, Transform, Vector};
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};

use crate::{direction::Direction, line_segment::LineSegment, Distance};

#[derive(
    Copy, Clone, Debug, Deserialize, Serialize, PathSerialize, PathIntrospect, PathDeserialize,
)]
pub struct Line<Frame, const DIMENSION: usize> {
    pub point: Point<Frame, DIMENSION>,
    pub direction: Vector<Frame, DIMENSION>,
}

pub type Line2<Frame> = Line<Frame, 2>;
pub type Line3<Frame> = Line<Frame, 3>;

impl<Frame> Line2<Frame> {
    pub fn slope(&self) -> f32 {
        self.direction.y() / self.direction.x()
    }

    pub fn y_axis_intercept(&self) -> f32 {
        self.point.y() - (self.point.x() * self.slope())
    }

    pub fn is_above(&self, point: Point2<Frame>) -> bool {
        self.signed_distance_to_point(point) >= 0.0
    }

    pub fn signed_distance_to_point(&self, point: Point2<Frame>) -> f32 {
        let normal_vector = Direction::Counterclockwise
            .rotate_vector_90_degrees(self.direction)
            .normalize();
        normal_vector.dot(point - self.point)
    }

    pub fn intersection(&self, other: &Line2<Frame>) -> Point2<Frame> {
        let point_difference = self.point - other.point;

        let direction_factor = (point_difference.x() * self.direction.y()
            - point_difference.y() * self.direction.x())
            / (self.direction.y() * other.direction.x() - self.direction.x() * other.direction.y());

        other.point + other.direction * direction_factor
    }

    pub fn project_onto_along_y_axis(&self, point: Point2<Frame>) -> f32 {
        let rise = (point.x() - self.point.x()) * self.slope();
        rise + self.point.y()
    }
}

impl<Frame, const DIMENSION: usize> Line<Frame, DIMENSION> {
    pub fn from_points(point1: Point<Frame, DIMENSION>, point2: Point<Frame, DIMENSION>) -> Self {
        Self {
            point: point1,
            direction: point2 - point1,
        }
    }

    pub fn closest_point(&self, point: Point<Frame, DIMENSION>) -> Point<Frame, DIMENSION> {
        self.point
            + (self.direction * self.direction.dot(point - self.point)
                / self.direction.norm_squared())
    }
}

impl<Frame, const DIMENSION: usize> Distance<Point<Frame, DIMENSION>> for Line<Frame, DIMENSION> {
    fn squared_distance_to(&self, point: Point<Frame, DIMENSION>) -> f32 {
        let closest_point = self.closest_point(point);
        distance_squared(closest_point, point)
    }
}

impl<Frame, const DIMENSION: usize> Default for Line<Frame, DIMENSION> {
    fn default() -> Self {
        Self {
            point: Default::default(),
            direction: Vector::zeros(),
        }
    }
}

impl<From, To, const DIMENSION: usize, Inner> Mul<Line<From, DIMENSION>>
    for Transform<From, To, Inner>
where
    Self: Mul<Point<From, DIMENSION>, Output = Point<To, DIMENSION>>
        + Mul<Vector<From, DIMENSION>, Output = Vector<To, DIMENSION>>
        + Copy,
{
    type Output = Line<To, DIMENSION>;

    fn mul(self, line: Line<From, DIMENSION>) -> Self::Output {
        Line {
            point: self * line.point,
            direction: self * line.direction,
        }
    }
}

impl<Frame, const DIMENSION: usize> PartialEq for Line<Frame, DIMENSION> {
    fn eq(&self, other: &Self) -> bool {
        self.point == other.point && self.direction == other.direction
    }
}

impl<Frame, const DIMENSION: usize> AbsDiffEq for Line<Frame, DIMENSION> {
    type Epsilon = <f32 as AbsDiffEq>::Epsilon;

    fn default_epsilon() -> Self::Epsilon {
        <f32 as AbsDiffEq>::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.point.abs_diff_eq(&other.point, epsilon)
            && self.direction.abs_diff_eq(&other.direction, epsilon)
    }
}

impl<Frame, const DIMENSION: usize> RelativeEq for Line<Frame, DIMENSION> {
    fn default_max_relative() -> Self::Epsilon {
        <f32 as RelativeEq>::default_max_relative()
    }

    fn relative_eq(
        &self,
        other: &Self,
        epsilon: Self::Epsilon,
        max_relative: Self::Epsilon,
    ) -> bool {
        self.point.relative_eq(&other.point, epsilon, max_relative)
            && self
                .direction
                .relative_eq(&other.direction, epsilon, max_relative)
    }
}

impl<Frame> From<LineSegment<Frame>> for Line2<Frame> {
    fn from(line_segment: LineSegment<Frame>) -> Self {
        Self {
            point: line_segment.0,
            direction: line_segment.1 - line_segment.0,
        }
    }
}
