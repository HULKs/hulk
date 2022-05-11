use std::{f32::consts::PI, ops::Mul};

use approx::{AbsDiffEq, RelativeEq};
use nalgebra::{Isometry, Point, Point2, UnitComplex};
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
pub struct Line<const D: usize>(pub Point<f32, D>, pub Point<f32, D>);

pub type Line2 = Line<2>;

impl Line2 {
    pub fn slope(&self) -> f32 {
        let difference = self.0 - self.1;
        difference.y / difference.x
    }

    pub fn y_axis_intercept(&self) -> f32 {
        self.0.y - (self.0.x * self.slope())
    }

    pub fn is_above(&self, point: Point2<f32>) -> bool {
        let rise = (point.x - self.0.x) * self.slope();
        point.y >= rise + self.0.y
    }
}

impl<const D: usize> Line<D> {
    pub fn project_point(&self, point: Point<f32, D>) -> Point<f32, D> {
        let difference_on_line = self.1 - self.0;
        let difference_to_point = point - self.0;
        self.0
            + (difference_on_line * difference_on_line.dot(&difference_to_point)
                / difference_on_line.norm_squared())
    }

    pub fn squared_distance_to_segment(&self, point: Point<f32, D>) -> f32 {
        let difference_on_line = self.1 - self.0;
        let difference_to_point = point - self.0;
        let t = difference_to_point.dot(&difference_on_line) / difference_on_line.norm_squared();
        if t <= 0.0 {
            (point - self.0).norm_squared()
        } else if t >= 1.0 {
            (point - self.1).norm_squared()
        } else {
            (point - (self.0 + difference_on_line * t)).norm_squared()
        }
    }

    pub fn squared_distance_to_point(&self, point: Point<f32, D>) -> f32 {
        let closest_point = self.project_point(point);
        (closest_point - point).norm_squared()
    }

    pub fn distance_to_point(&self, point: Point<f32, D>) -> f32 {
        self.squared_distance_to_point(point).sqrt()
    }

    pub fn is_orthogonal(&self, other: &Line<D>, epsilon: f32) -> bool {
        let self_direction = (self.1 - self.0).normalize();
        let other_direction = (other.1 - other.0).normalize();
        (self_direction.dot(&other_direction).acos().abs() - PI / 2.0).abs() < epsilon
    }

    pub fn length(&self) -> f32 {
        (self.1 - self.0).norm()
    }
}

impl<const D: usize> Mul<Line<D>> for Isometry<f32, UnitComplex<f32>, D>
where
    Isometry<f32, UnitComplex<f32>, D>: Mul<Point<f32, D>, Output = Point<f32, D>>,
{
    type Output = Line<D>;

    fn mul(self, rhs: Line<D>) -> Self::Output {
        Line(self * rhs.0, self * rhs.1)
    }
}

impl<const D: usize> PartialEq for Line<D> {
    fn eq(&self, other: &Self) -> bool {
        (self.0 == other.0 && self.1 == other.1) || (self.0 == other.1 && self.1 == other.0)
    }
}

impl<const D: usize> AbsDiffEq for Line<D> {
    type Epsilon = <f32 as AbsDiffEq>::Epsilon;

    fn default_epsilon() -> Self::Epsilon {
        <f32 as AbsDiffEq>::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.0.abs_diff_eq(&other.0, epsilon) && self.1.abs_diff_eq(&other.1, epsilon)
    }
}

impl<const D: usize> RelativeEq for Line<D> {
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
