use std::{
    f32::consts::{FRAC_PI_2, PI},
    ops::Mul,
};

use approx::{AbsDiffEq, RelativeEq};
use nalgebra::{
    center, distance, distance_squared, point, vector, Isometry, Point, Point2, UnitComplex,
    Vector2,
};
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
pub struct Line<const DIMENSION: usize>(pub Point<f32, DIMENSION>, pub Point<f32, DIMENSION>);

pub type Line2 = Line<2>;

impl Line2 {
    pub fn signed_acute_angle(&self, other: Self) -> f32 {
        let self_direction = self.1 - self.0;
        let other_direction = other.1 - other.0;
        signed_acute_angle(&self_direction, &other_direction)
    }

    pub fn angle(&self, other: Self) -> f32 {
        (self.1 - self.0).angle(&(other.1 - other.0))
    }

    pub fn signed_acute_angle_to_orthogonal(&self, other: Self) -> f32 {
        let self_direction = self.1 - self.0;
        let other_direction = other.1 - other.0;
        let orthogonal_other_direction = vector![other_direction.y, -other_direction.x];
        signed_acute_angle(&self_direction, &orthogonal_other_direction)
    }

    pub fn is_orthogonal(&self, other: Self, epsilon: f32) -> bool {
        self.signed_acute_angle_to_orthogonal(other) < epsilon
    }

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

    pub fn signed_distance_to_point(&self, point: Point2<f32>) -> f32 {
        let line_vector = self.1 - self.0;
        let normal_vector = vector![-line_vector.y, line_vector.x].normalize();
        normal_vector.dot(&point.coords) - normal_vector.dot(&self.0.coords)
    }

    pub fn project_onto_segment(&self, point: Point2<f32>) -> Point2<f32> {
        let difference_on_line = self.1 - self.0;
        let difference_to_point = point - self.0;
        let t = difference_to_point.dot(&difference_on_line) / difference_on_line.norm_squared();
        if t <= 0.0 {
            self.0
        } else if t >= 1.0 {
            self.1
        } else {
            self.0 + difference_on_line * t
        }
    }

    pub fn intersection(&self, other: &Line2) -> Point2<f32> {
        let x1 = self.0.coords[0];
        let y1 = self.0.coords[1];
        let x2 = self.1.coords[0];
        let y2 = self.1.coords[1];
        let x3 = other.0.coords[0];
        let y3 = other.0.coords[1];
        let x4 = other.1.coords[0];
        let y4 = other.1.coords[1];

        point!(
            ((((x1 * y2) - (y1 * x2)) * (x3 - x4)) - ((x1 - x2) * ((x3 * y4) - (y3 * x4))))
                / (((x1 - x2) * (y3 - y4)) - ((y1 - y2) * (x3 - x4))),
            ((((x1 * y2) - (y1 * x2)) * (y3 - y4)) - ((y1 - y2) * ((x3 * y4) - (y3 * x4))))
                / (((x1 - x2) * (y3 - y4)) - ((y1 - y2) * (x3 - x4)))
        )
    }
}

fn signed_acute_angle(first: &Vector2<f32>, second: &Vector2<f32>) -> f32 {
    let difference = UnitComplex::rotation_between(first, second).angle();
    if difference > FRAC_PI_2 {
        difference - PI
    } else if difference < -FRAC_PI_2 {
        difference + PI
    } else {
        difference
    }
}

impl<const DIMENSION: usize> Line<DIMENSION> {
    pub fn project_point(&self, point: Point<f32, DIMENSION>) -> Point<f32, DIMENSION> {
        let difference_on_line = self.1 - self.0;
        let difference_to_point = point - self.0;
        self.0
            + (difference_on_line * difference_on_line.dot(&difference_to_point)
                / difference_on_line.norm_squared())
    }

    pub fn squared_distance_to_segment(&self, point: Point<f32, DIMENSION>) -> f32 {
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

    pub fn squared_distance_to_point(&self, point: Point<f32, DIMENSION>) -> f32 {
        let closest_point = self.project_point(point);
        distance_squared(&closest_point, &point)
    }

    pub fn distance_to_point(&self, point: Point<f32, DIMENSION>) -> f32 {
        self.squared_distance_to_point(point).sqrt()
    }

    pub fn length(&self) -> f32 {
        distance(&self.0, &self.1)
    }

    pub fn center(&self) -> Point<f32, DIMENSION> {
        center(&self.0, &self.1)
    }
}

impl<const DIMENSION: usize> Mul<Line<DIMENSION>> for Isometry<f32, UnitComplex<f32>, DIMENSION>
where
    Isometry<f32, UnitComplex<f32>, DIMENSION>:
        Mul<Point<f32, DIMENSION>, Output = Point<f32, DIMENSION>>,
{
    type Output = Line<DIMENSION>;

    fn mul(self, right: Line<DIMENSION>) -> Self::Output {
        Line(self * right.0, self * right.1)
    }
}

impl<const DIMENSION: usize> PartialEq for Line<DIMENSION> {
    fn eq(&self, other: &Self) -> bool {
        (self.0 == other.0 && self.1 == other.1) || (self.0 == other.1 && self.1 == other.0)
    }
}

impl<const DIMENSION: usize> AbsDiffEq for Line<DIMENSION> {
    type Epsilon = <f32 as AbsDiffEq>::Epsilon;

    fn default_epsilon() -> Self::Epsilon {
        <f32 as AbsDiffEq>::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.0.abs_diff_eq(&other.0, epsilon) && self.1.abs_diff_eq(&other.1, epsilon)
    }
}

impl<const DIMENSION: usize> RelativeEq for Line<DIMENSION> {
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

#[cfg(test)]
mod tests {
    use std::f32::consts::FRAC_PI_4;

    use approx::assert_relative_eq;

    use super::*;

    #[test]
    fn correct_acute_signed_angle() {
        #[derive(Debug)]
        struct Case {
            self_line: Line2,
            other_line: Line2,
            expected_angle: f32,
        }

        let thirty_degree = 30.0_f32.to_radians();
        let sixty_degree = 60.0_f32.to_radians();
        let cases = [
            Case {
                self_line: Line(point![0.0, 0.0], point![42.0, 0.0]),
                other_line: Line(point![0.0, 0.0], point![42.0, 0.0]),
                expected_angle: 0.0,
            },
            Case {
                self_line: Line(point![0.0, 0.0], point![42.0, 0.0]),
                other_line: Line(point![0.0, 0.0], point![42.0, 42.0]),
                expected_angle: FRAC_PI_4,
            },
            Case {
                self_line: Line(point![0.0, 0.0], point![42.0, 42.0]),
                other_line: Line(point![0.0, 0.0], point![42.0, 0.0]),
                expected_angle: -FRAC_PI_4,
            },
            Case {
                self_line: Line(point![0.0, 0.0], point![42.0, 0.0]),
                other_line: Line(point![0.0, 0.0], point![42.0, -42.0]),
                expected_angle: -FRAC_PI_4,
            },
            Case {
                self_line: Line(point![0.0, 0.0], point![42.0, -42.0]),
                other_line: Line(point![0.0, 0.0], point![42.0, 0.0]),
                expected_angle: FRAC_PI_4,
            },
            Case {
                self_line: Line(
                    point![0.0, 0.0],
                    point![(-thirty_degree).cos(), (-thirty_degree).sin()],
                ),
                other_line: Line(
                    point![0.0, 0.0],
                    point![thirty_degree.cos(), thirty_degree.sin()],
                ),
                expected_angle: sixty_degree,
            },
            Case {
                self_line: Line(
                    point![0.0, 0.0],
                    point![thirty_degree.cos(), thirty_degree.sin()],
                ),
                other_line: Line(
                    point![0.0, 0.0],
                    point![(-thirty_degree).cos(), (-thirty_degree).sin()],
                ),
                expected_angle: -sixty_degree,
            },
            Case {
                self_line: Line(
                    point![0.0, 0.0],
                    point![(-sixty_degree).cos(), (-sixty_degree).sin()],
                ),
                other_line: Line(
                    point![0.0, 0.0],
                    point![sixty_degree.cos(), sixty_degree.sin()],
                ),
                expected_angle: -sixty_degree,
            },
            Case {
                self_line: Line(
                    point![0.0, 0.0],
                    point![sixty_degree.cos(), sixty_degree.sin()],
                ),
                other_line: Line(
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
                    self_line: Line(case.self_line.1, case.self_line.0),
                    other_line: case.other_line,
                    expected_angle: case.expected_angle,
                },
                Case {
                    self_line: case.self_line,
                    other_line: Line(case.other_line.1, case.other_line.0),
                    expected_angle: case.expected_angle,
                },
                Case {
                    self_line: Line(case.self_line.1, case.self_line.0),
                    other_line: Line(case.other_line.1, case.other_line.0),
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
