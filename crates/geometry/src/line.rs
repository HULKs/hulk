use std::{
    f32::consts::{FRAC_PI_2, PI},
    ops::Mul,
};

use approx::{AbsDiffEq, RelativeEq};
use linear_algebra::{
    center, distance, distance_squared, point, vector, Point, Point2, Rotation2, Transform, Vector2,
};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

#[derive(Copy, Clone, Default, Debug, Deserialize, Serialize, SerializeHierarchy)]
#[serde(bound = "")]
pub struct Line<Frame, const DIMENSION: usize> {
    pub first: Point<Frame, DIMENSION>,
    pub second: Point<Frame, DIMENSION>,
}

impl<Frame, const DIMENSION: usize> Line<Frame, DIMENSION> {
    pub fn new(first: Point<Frame, DIMENSION>, second: Point<Frame, DIMENSION>) -> Self {
        Self { first, second }
    }
}

pub type Line2<Frame> = Line<Frame, 2>;
pub type Line3<Frame> = Line<Frame, 3>;

impl<Frame> Line2<Frame> {
    pub fn signed_acute_angle(&self, other: Self) -> f32 {
        let self_direction = self.second - self.first;
        let other_direction = other.second - other.first;
        signed_acute_angle(self_direction, other_direction)
    }

    pub fn angle(&self, other: Self) -> f32 {
        (self.second - self.first).angle(other.second - other.first)
    }

    pub fn signed_acute_angle_to_orthogonal(&self, other: Self) -> f32 {
        let self_direction = self.second - self.first;
        let other_direction = other.second - other.first;
        let orthogonal_other_direction = vector![other_direction.y(), -other_direction.x()];
        signed_acute_angle(self_direction, orthogonal_other_direction)
    }

    pub fn is_orthogonal(&self, other: Self, epsilon: f32) -> bool {
        self.signed_acute_angle_to_orthogonal(other) < epsilon
    }

    pub fn slope(&self) -> f32 {
        let difference = self.first - self.second;
        difference.y() / difference.x()
    }

    pub fn y_axis_intercept(&self) -> f32 {
        self.first.y() - (self.first.x() * self.slope())
    }

    pub fn is_above(&self, point: Point2<Frame>) -> bool {
        let rise = (point.x() - self.first.x()) * self.slope();
        point.y() >= rise + self.first.y()
    }

    pub fn signed_distance_to_point(&self, point: Point2<Frame>) -> f32 {
        let line_vector = self.second - self.first;
        let normal_vector = vector![-line_vector.y(), line_vector.x()].normalize();
        normal_vector.dot(point.coords()) - normal_vector.dot(self.first.coords())
    }

    pub fn project_onto_segment(&self, point: Point2<Frame>) -> Point2<Frame> {
        let difference_on_line = self.second - self.first;
        let difference_to_point = point - self.first;
        let t = difference_to_point.dot(difference_on_line) / difference_on_line.norm_squared();
        if t <= 0.0 {
            self.first
        } else if t >= 1.0 {
            self.second
        } else {
            self.first + difference_on_line * t
        }
    }

    pub fn intersection(&self, other: &Line2<Frame>) -> Point2<Frame> {
        let x1 = self.first.x();
        let y1 = self.first.y();
        let x2 = self.second.x();
        let y2 = self.second.y();
        let x3 = other.first.x();
        let y3 = other.first.y();
        let x4 = other.second.x();
        let y4 = other.second.y();

        point!(
            ((((x1 * y2) - (y1 * x2)) * (x3 - x4)) - ((x1 - x2) * ((x3 * y4) - (y3 * x4))))
                / (((x1 - x2) * (y3 - y4)) - ((y1 - y2) * (x3 - x4))),
            ((((x1 * y2) - (y1 * x2)) * (y3 - y4)) - ((y1 - y2) * ((x3 * y4) - (y3 * x4))))
                / (((x1 - x2) * (y3 - y4)) - ((y1 - y2) * (x3 - x4)))
        )
    }
}

fn signed_acute_angle<Frame>(first: Vector2<Frame>, second: Vector2<Frame>) -> f32 {
    let difference = Rotation2::rotation_between(first, second).angle();
    if difference > FRAC_PI_2 {
        difference - PI
    } else if difference < -FRAC_PI_2 {
        difference + PI
    } else {
        difference
    }
}

impl<Frame, const DIMENSION: usize> Line<Frame, DIMENSION> {
    pub fn project_point(&self, point: Point<Frame, DIMENSION>) -> Point<Frame, DIMENSION> {
        let difference_on_line = self.second - self.first;
        let difference_to_point = point - self.first;
        self.first
            + (difference_on_line * difference_on_line.dot(difference_to_point)
                / difference_on_line.norm_squared())
    }

    pub fn squared_distance_to_segment(&self, point: Point<Frame, DIMENSION>) -> f32 {
        let difference_on_line = self.second - self.first;
        let difference_to_point = point - self.first;
        let t = difference_to_point.dot(difference_on_line) / difference_on_line.norm_squared();
        if t <= 0.0 {
            (point - self.first).norm_squared()
        } else if t >= 1.0 {
            (point - self.second).norm_squared()
        } else {
            (point - (self.first + difference_on_line * t)).norm_squared()
        }
    }

    pub fn squared_distance_to_point(&self, point: Point<Frame, DIMENSION>) -> f32 {
        let closest_point = self.project_point(point);
        distance_squared(closest_point, point)
    }

    pub fn distance_to_point(&self, point: Point<Frame, DIMENSION>) -> f32 {
        self.squared_distance_to_point(point).sqrt()
    }

    pub fn length(&self) -> f32 {
        distance(self.first, self.second)
    }

    pub fn center(&self) -> Point<Frame, DIMENSION> {
        center(self.first, self.second)
    }
}

impl<From, To, const DIMENSION: usize, Inner> Mul<Line<From, DIMENSION>>
    for Transform<From, To, Inner>
where
    Self: Mul<Point<From, DIMENSION>, Output = Point<To, DIMENSION>> + Copy,
{
    type Output = Line<To, DIMENSION>;

    fn mul(self, right: Line<From, DIMENSION>) -> Self::Output {
        Line {
            first: self * right.first,
            second: self * right.second,
        }
    }
}

impl<Frame, const DIMENSION: usize> PartialEq for Line<Frame, DIMENSION> {
    fn eq(&self, other: &Self) -> bool {
        (self.first == other.first && self.second == other.second)
            || (self.first == other.second && self.second == other.first)
    }
}

impl<Frame, const DIMENSION: usize> AbsDiffEq for Line<Frame, DIMENSION> {
    type Epsilon = <f32 as AbsDiffEq>::Epsilon;

    fn default_epsilon() -> Self::Epsilon {
        <f32 as AbsDiffEq>::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.first.abs_diff_eq(&other.first, epsilon)
            && self.second.abs_diff_eq(&other.second, epsilon)
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
        self.first.relative_eq(&other.first, epsilon, max_relative)
            && self
                .second
                .relative_eq(&other.second, epsilon, max_relative)
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::FRAC_PI_4;

    use approx::assert_relative_eq;

    use super::*;

    #[derive(Clone, Copy, Debug)]
    struct SomeFrame;

    #[test]
    fn correct_acute_signed_angle() {
        #[derive(Debug)]
        struct Case {
            self_line: Line2<SomeFrame>,
            other_line: Line2<SomeFrame>,
            expected_angle: f32,
        }

        let thirty_degree = 30.0_f32.to_radians();
        let sixty_degree = 60.0_f32.to_radians();
        let cases = [
            Case {
                self_line: Line::new(point![0.0, 0.0], point![42.0, 0.0]),
                other_line: Line::new(point![0.0, 0.0], point![42.0, 0.0]),
                expected_angle: 0.0,
            },
            Case {
                self_line: Line::new(point![0.0, 0.0], point![42.0, 0.0]),
                other_line: Line::new(point![0.0, 0.0], point![42.0, 42.0]),
                expected_angle: FRAC_PI_4,
            },
            Case {
                self_line: Line::new(point![0.0, 0.0], point![42.0, 42.0]),
                other_line: Line::new(point![0.0, 0.0], point![42.0, 0.0]),
                expected_angle: -FRAC_PI_4,
            },
            Case {
                self_line: Line::new(point![0.0, 0.0], point![42.0, 0.0]),
                other_line: Line::new(point![0.0, 0.0], point![42.0, -42.0]),
                expected_angle: -FRAC_PI_4,
            },
            Case {
                self_line: Line::new(point![0.0, 0.0], point![42.0, -42.0]),
                other_line: Line::new(point![0.0, 0.0], point![42.0, 0.0]),
                expected_angle: FRAC_PI_4,
            },
            Case {
                self_line: Line::new(
                    point![0.0, 0.0],
                    point![(-thirty_degree).cos(), (-thirty_degree).sin()],
                ),
                other_line: Line::new(
                    point![0.0, 0.0],
                    point![thirty_degree.cos(), thirty_degree.sin()],
                ),
                expected_angle: sixty_degree,
            },
            Case {
                self_line: Line::new(
                    point![0.0, 0.0],
                    point![thirty_degree.cos(), thirty_degree.sin()],
                ),
                other_line: Line::new(
                    point![0.0, 0.0],
                    point![(-thirty_degree).cos(), (-thirty_degree).sin()],
                ),
                expected_angle: -sixty_degree,
            },
            Case {
                self_line: Line::new(
                    point![0.0, 0.0],
                    point![(-sixty_degree).cos(), (-sixty_degree).sin()],
                ),
                other_line: Line::new(
                    point![0.0, 0.0],
                    point![sixty_degree.cos(), sixty_degree.sin()],
                ),
                expected_angle: -sixty_degree,
            },
            Case {
                self_line: Line::new(
                    point![0.0, 0.0],
                    point![sixty_degree.cos(), sixty_degree.sin()],
                ),
                other_line: Line::new(
                    point![0.0, 0.0],
                    point![(-sixty_degree).cos(), (-sixty_degree).sin()],
                ),
                expected_angle: sixty_degree,
            },
        ]
        .into_iter()
        .flat_map(|case| {
            [
                Case {
                    self_line: case.self_line,
                    other_line: case.other_line,
                    expected_angle: case.expected_angle,
                },
                Case {
                    self_line: Line::new(case.self_line.second, case.self_line.first),
                    other_line: case.other_line,
                    expected_angle: case.expected_angle,
                },
                Case {
                    self_line: case.self_line,
                    other_line: Line::new(case.other_line.second, case.other_line.first),
                    expected_angle: case.expected_angle,
                },
                Case {
                    self_line: Line::new(case.self_line.second, case.self_line.first),
                    other_line: Line::new(case.other_line.second, case.other_line.first),
                    expected_angle: case.expected_angle,
                },
            ]
        });

        for case in cases {
            dbg!(&case);
            assert_relative_eq!(
                case.self_line.signed_acute_angle(case.other_line),
                case.expected_angle,
                epsilon = 0.000001,
            );
        }
    }
}
