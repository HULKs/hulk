use std::ops::Mul;

use approx::{AbsDiffEq, RelativeEq};
use serde::{Deserialize, Serialize};

use linear_algebra::{distance_squared, point, Point, Point2, Transform};
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};

use crate::{direction::Direction, line_segment::LineSegment, Distance};

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
pub struct Line<Frame, const DIMENSION: usize>(
    pub Point<Frame, DIMENSION>,
    pub Point<Frame, DIMENSION>,
);

pub type Line2<Frame> = Line<Frame, 2>;
pub type Line3<Frame> = Line<Frame, 3>;

impl<Frame> Line2<Frame> {
    pub fn slope(&self) -> f32 {
        let difference = self.0 - self.1;
        difference.y() / difference.x()
    }

    pub fn y_axis_intercept(&self) -> f32 {
        self.0.y() - (self.0.x() * self.slope())
    }

    pub fn is_above(&self, point: Point2<Frame>) -> bool {
        self.signed_distance_to_point(point) >= 0.0
    }

    pub fn signed_distance_to_point(&self, point: Point2<Frame>) -> f32 {
        let line_vector = self.1 - self.0;
        let normal_vector = Direction::Counterclockwise
            .rotate_vector_90_degrees(line_vector)
            .normalize();
        normal_vector.dot(point.coords()) - normal_vector.dot(self.0.coords())
    }

    pub fn intersection(&self, other: &Line2<Frame>) -> Point2<Frame> {
        let x1 = self.0.x();
        let y1 = self.0.y();
        let x2 = self.1.x();
        let y2 = self.1.y();
        let x3 = other.0.x();
        let y3 = other.0.y();
        let x4 = other.1.x();
        let y4 = other.1.y();

        point!(
            ((((x1 * y2) - (y1 * x2)) * (x3 - x4)) - ((x1 - x2) * ((x3 * y4) - (y3 * x4))))
                / (((x1 - x2) * (y3 - y4)) - ((y1 - y2) * (x3 - x4))),
            ((((x1 * y2) - (y1 * x2)) * (y3 - y4)) - ((y1 - y2) * ((x3 * y4) - (y3 * x4))))
                / (((x1 - x2) * (y3 - y4)) - ((y1 - y2) * (x3 - x4)))
        )
    }
}

impl<Frame, const DIMENSION: usize> Line<Frame, DIMENSION> {
    pub fn closest_point(&self, point: Point<Frame, DIMENSION>) -> Point<Frame, DIMENSION> {
        let difference_on_line = self.1 - self.0;
        let difference_to_point = point - self.0;
        self.0
            + (difference_on_line * difference_on_line.dot(difference_to_point)
                / difference_on_line.norm_squared())
    }
}

impl<Frame, const DIMENSION: usize> Distance<Point<Frame, DIMENSION>> for Line<Frame, DIMENSION> {
    fn squared_distance_to(&self, point: Point<Frame, DIMENSION>) -> f32 {
        let closest_point = self.closest_point(point);
        distance_squared(closest_point, point)
    }
}

impl<From, To, const DIMENSION: usize, Inner> Mul<Line<From, DIMENSION>>
    for Transform<From, To, Inner>
where
    Self: Mul<Point<From, DIMENSION>, Output = Point<To, DIMENSION>> + Copy,
{
    type Output = Line<To, DIMENSION>;

    fn mul(self, right: Line<From, DIMENSION>) -> Self::Output {
        Line(self * right.0, self * right.1)
    }
}

impl<Frame, const DIMENSION: usize> PartialEq for Line<Frame, DIMENSION> {
    fn eq(&self, other: &Self) -> bool {
        (self.0 == other.0 && self.1 == other.1) || (self.0 == other.1 && self.1 == other.0)
    }
}

impl<Frame, const DIMENSION: usize> AbsDiffEq for Line<Frame, DIMENSION> {
    type Epsilon = <f32 as AbsDiffEq>::Epsilon;

    fn default_epsilon() -> Self::Epsilon {
        <f32 as AbsDiffEq>::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.0.abs_diff_eq(&other.0, epsilon) && self.1.abs_diff_eq(&other.1, epsilon)
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
        self.0.relative_eq(&other.0, epsilon, max_relative)
            && self.1.relative_eq(&other.1, epsilon, max_relative)
    }
}

impl<Frame> From<LineSegment<Frame>> for Line2<Frame> {
    fn from(line_segment: LineSegment<Frame>) -> Self {
        Self(line_segment.0, line_segment.1)
    }
}
