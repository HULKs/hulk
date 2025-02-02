use std::{
    cmp::PartialEq,
    f32::consts::{FRAC_PI_2, PI},
    ops::Mul,
};

use approx::{AbsDiffEq, RelativeEq};
use serde::{Deserialize, Serialize};

use linear_algebra::{
    center, distance, distance_squared, vector, Point2, Rotation2, Transform, Vector2,
};
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};

use crate::{angle::Angle, arc::Arc, direction::Direction, Distance};

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Deserialize,
    Serialize,
    PartialEq,
    PathSerialize,
    PathIntrospect,
    PathDeserialize,
)]
pub struct LineSegment<Frame>(pub Point2<Frame>, pub Point2<Frame>);

impl<Frame> LineSegment<Frame> {
    pub fn new(start: Point2<Frame>, end: Point2<Frame>) -> Self {
        Self(start, end)
    }
    pub fn flip(self) -> Self {
        Self(self.1, self.0)
    }

    pub fn length(&self) -> f32 {
        distance(self.0, self.1)
    }

    pub fn length_squared(&self) -> f32 {
        distance_squared(self.0, self.1)
    }

    pub fn center(&self) -> Point2<Frame> {
        center(self.0, self.1)
    }

    pub fn signed_distance_to_point(&self, point: Point2<Frame>) -> f32 {
        let line_vector = self.1 - self.0;
        let normal_vector = Direction::Counterclockwise
            .rotate_vector_90_degrees(line_vector)
            .normalize();
        normal_vector.dot(&point.coords()) - normal_vector.dot(&self.0.coords())
    }

    pub fn signed_acute_angle(&self, other: Self) -> f32 {
        let self_direction = self.1 - self.0;
        let other_direction = other.1 - other.0;
        signed_acute_angle(self_direction, other_direction)
    }

    pub fn angle(&self, other: Self) -> f32 {
        (self.1 - self.0).angle(&(other.1 - other.0))
    }

    pub fn signed_acute_angle_to_orthogonal(&self, other: Self) -> f32 {
        let self_direction = self.1 - self.0;
        let other_direction = other.1 - other.0;
        let orthogonal_other_direction =
            Direction::Clockwise.rotate_vector_90_degrees(other_direction);
        signed_acute_angle(self_direction, orthogonal_other_direction)
    }

    pub fn is_orthogonal(&self, other: Self, epsilon: f32) -> bool {
        self.signed_acute_angle_to_orthogonal(other).abs() < epsilon
    }

    pub fn projection_factor(&self, point: &Point2<Frame>) -> f32 {
        let projection = (point - self.0).dot(&(self.1 - self.0));

        projection / self.length_squared()
    }

    pub fn closest_point(&self, point: Point2<Frame>) -> Point2<Frame> {
        let projected_factor = self.projection_factor(&point).clamp(0.0, 1.0);
        self.0 + (self.1 - self.0) * projected_factor
    }

    /// Reference: <https://algotree.org/algorithms/computational_geometry/line_segment_intersection/>
    pub fn intersects_line_segment(&self, other: LineSegment<Frame>) -> bool {
        let orientation_other_points_to_self =
            (self.get_direction(other.0), self.get_direction(other.1));

        match orientation_other_points_to_self {
            (Direction::Counterclockwise, Direction::Counterclockwise)
            | (Direction::Clockwise, Direction::Clockwise) => false,

            (Direction::Colinear, Direction::Colinear) => {
                self.overlaps_collinear_line_segment(other)
            }

            _ => {
                let orientation_self_points_to_other =
                    (other.get_direction(self.0), other.get_direction(self.1));

                orientation_self_points_to_other.0 != orientation_self_points_to_other.1
                    || orientation_self_points_to_other.0 == Direction::Colinear
            }
        }
    }

    fn overlaps_collinear_line_segment(&self, other: LineSegment<Frame>) -> bool {
        self.bounding_box_contains(other.0)
            || self.bounding_box_contains(other.1)
            || other.bounding_box_contains(self.0)
            || other.bounding_box_contains(self.1)
    }

    fn bounding_box_contains(&self, point: Point2<Frame>) -> bool {
        point.x() > f32::min(self.0.x(), self.1.x())
            && point.x() < f32::max(self.0.x(), self.1.x())
            && point.y() < f32::max(self.0.y(), self.1.y())
            && point.y() > f32::min(self.0.y(), self.1.y())
    }

    pub fn get_direction(&self, point: Point2<Frame>) -> Direction {
        let direction_vector = self.1 - self.0;
        let clockwise_normal_vector = vector![direction_vector.y(), -direction_vector.x()];
        let directed_cathetus = clockwise_normal_vector.dot(&(point - self.0));

        match directed_cathetus {
            f if f == 0.0 => Direction::Colinear,
            f if f > 0.0 => Direction::Clockwise,
            f if f < 0.0 => Direction::Counterclockwise,
            f => panic!("directed cathetus was not a real number: {f}"),
        }
    }

    pub fn overlaps_arc(&self, arc: Arc<Frame>) -> bool {
        if self.distance_to(arc.circle.center) >= arc.circle.radius {
            return false;
        }

        let direction = self.1 - self.0;
        let normed_direction = direction.normalize();

        let projection = (arc.circle.center - self.0).dot(&normed_direction);
        let base_point = self.0 + normed_direction * projection;

        let center_to_base_length = (base_point - arc.circle.center).norm();
        let base_to_intersection_length =
            f32::sqrt(arc.circle.radius.powi(2) - center_to_base_length.powi(2));

        let intersection_point1 = base_point + normed_direction * base_to_intersection_length;
        let intersection_point2 = base_point - normed_direction * base_to_intersection_length;

        let angle_start_to_end = arc.start.angle_to(arc.end, arc.direction);

        [intersection_point1, intersection_point2]
            .into_iter()
            .filter(|intersection_point| {
                (0.0..1.0).contains(&self.projection_factor(intersection_point))
            })
            .any(|intersection_point| {
                let angle_to_intersection_point =
                    Angle::from_direction(intersection_point - arc.circle.center);
                let angle_start_to_intersection_point = arc
                    .start
                    .angle_to(angle_to_intersection_point, arc.direction);

                angle_start_to_intersection_point.0 < angle_start_to_end.0
            })
    }

    pub fn translate(&self, translation: Vector2<Frame>) -> Self {
        Self::new(self.0 + translation, self.1 + translation)
    }
}

impl<From, To, Inner> Mul<LineSegment<From>> for Transform<From, To, Inner>
where
    Self: Mul<Point2<From>, Output = Point2<To>> + Copy,
{
    type Output = LineSegment<To>;

    fn mul(self, right: LineSegment<From>) -> Self::Output {
        LineSegment(self * right.0, self * right.1)
    }
}

impl<Frame> Distance<Point2<Frame>> for LineSegment<Frame> {
    fn squared_distance_to(&self, point: Point2<Frame>) -> f32 {
        distance_squared(point, self.closest_point(point))
    }
}

impl<Frame> AbsDiffEq for LineSegment<Frame>
where
    Frame: PartialEq,
{
    type Epsilon = f32;

    fn default_epsilon() -> Self::Epsilon {
        f32::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        Point2::abs_diff_eq(&other.0, &self.0, epsilon)
            && Point2::abs_diff_eq(&other.1, &self.1, epsilon)
    }
}

impl<Frame> RelativeEq for LineSegment<Frame>
where
    Frame: PartialEq,
{
    fn default_max_relative() -> f32 {
        f32::default_max_relative()
    }

    fn relative_eq(
        &self,
        other: &Self,
        epsilon: Self::Epsilon,
        max_relative: Self::Epsilon,
    ) -> bool {
        Point2::relative_eq(&self.0, &other.0, epsilon, max_relative)
            && Point2::relative_eq(&self.1, &other.1, epsilon, max_relative)
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

#[cfg(test)]
mod tests {
    use std::f32::consts::FRAC_PI_4;

    use approx::assert_relative_eq;
    use linear_algebra::point;

    use crate::circle::Circle;

    use super::*;

    #[derive(Debug, Clone, Copy)]
    struct SomeFrame;

    #[test]
    fn line_segment_lengths() {
        let line_segment = LineSegment::<SomeFrame>(Point2::origin(), point![0.0, 5.0]);
        assert_relative_eq!(line_segment.length(), 5.0);
        assert_relative_eq!(line_segment.length_squared(), 5.0 * 5.0);
        let diagonal = LineSegment::<SomeFrame>(point![-1.0, -1.0], point![1.0, 1.0]);
        assert_relative_eq!(diagonal.length(), 8.0_f32.sqrt());
        assert_relative_eq!(diagonal.length_squared(), 8.0);
    }

    #[test]
    fn shortest_distance_between_point_and_line_segment() {
        let line_segment = LineSegment::<SomeFrame>(point![-1.0, 0.0], point![1.0, 0.0]);

        assert_relative_eq!(0.0, line_segment.distance_to(point![-1.0, 0.0]));
        assert_relative_eq!(0.0, line_segment.distance_to(point![1.0, 0.0]));
        assert_relative_eq!(1.0, line_segment.distance_to(point![0.0, 1.0]));
        assert_relative_eq!(2.0_f32.sqrt(), line_segment.distance_to(point![2.0, -1.0]));
        assert_relative_eq!(0.5, line_segment.distance_to(point![-0.5, -0.5]));
    }

    fn test_all_permutations(
        reference_line_segment: LineSegment<SomeFrame>,
        line_segment: LineSegment<SomeFrame>,
        intersects: bool,
    ) {
        assert_eq!(
            reference_line_segment.intersects_line_segment(line_segment),
            intersects
        );
        assert_eq!(
            reference_line_segment.intersects_line_segment(line_segment.flip()),
            intersects
        );
        assert_eq!(
            reference_line_segment
                .flip()
                .intersects_line_segment(line_segment),
            intersects
        );
        assert_eq!(
            reference_line_segment
                .flip()
                .intersects_line_segment(line_segment.flip()),
            intersects
        );
        assert_eq!(
            line_segment.intersects_line_segment(reference_line_segment),
            intersects
        );
        assert_eq!(
            line_segment.intersects_line_segment(reference_line_segment.flip()),
            intersects
        );
        assert_eq!(
            line_segment
                .flip()
                .intersects_line_segment(reference_line_segment),
            intersects
        );
        assert_eq!(
            line_segment
                .flip()
                .intersects_line_segment(reference_line_segment.flip()),
            intersects
        );
    }

    #[test]
    fn parallel_lines_intersection() {
        let reference_line_segment = LineSegment(point![0.0, 0.0], point![1.0, 0.0]);
        let line_segment = LineSegment(point![1.0, 1.0], point![2.0, 1.0]);
        test_all_permutations(reference_line_segment, line_segment, false);
    }

    #[test]
    fn crossing_lines_intersection() {
        let reference_line_segment = LineSegment(point![0.0, 0.0], point![1.0, 0.0]);
        let line_segment = LineSegment(point![0.5, -1.0], point![0.5, 1.0]);
        test_all_permutations(reference_line_segment, line_segment, true);
    }

    #[test]
    fn t_shaped_lines_intersection() {
        let reference_line_segment = LineSegment(point![0.0, 0.0], point![1.0, 0.0]);
        let line_segment = LineSegment(point![1.1, -1.0], point![1.1, 1.0]);
        test_all_permutations(reference_line_segment, line_segment, false);
    }

    #[test]
    fn skew_lines_intersection() {
        let reference_line_segment = LineSegment(point![0.0, 0.0], point![1.0, 0.0]);
        let line_segment = LineSegment(point![5.0, 4.0], point![4.0, 5.0]);
        test_all_permutations(reference_line_segment, line_segment, false);
    }

    #[test]
    fn correct_acute_signed_angle() {
        #[derive(Debug)]
        struct Case {
            self_line: LineSegment<SomeFrame>,
            other_line: LineSegment<SomeFrame>,
            expected_angle: f32,
        }

        let thirty_degree = 30.0_f32.to_radians();
        let sixty_degree = 60.0_f32.to_radians();
        let cases = [
            Case {
                self_line: LineSegment(point![0.0, 0.0], point![42.0, 0.0]),
                other_line: LineSegment(point![0.0, 0.0], point![42.0, 0.0]),
                expected_angle: 0.0,
            },
            Case {
                self_line: LineSegment(point![0.0, 0.0], point![42.0, 0.0]),
                other_line: LineSegment(point![0.0, 0.0], point![42.0, 42.0]),
                expected_angle: FRAC_PI_4,
            },
            Case {
                self_line: LineSegment(point![0.0, 0.0], point![42.0, 42.0]),
                other_line: LineSegment(point![0.0, 0.0], point![42.0, 0.0]),
                expected_angle: -FRAC_PI_4,
            },
            Case {
                self_line: LineSegment(point![0.0, 0.0], point![42.0, 0.0]),
                other_line: LineSegment(point![0.0, 0.0], point![42.0, -42.0]),
                expected_angle: -FRAC_PI_4,
            },
            Case {
                self_line: LineSegment(point![0.0, 0.0], point![42.0, -42.0]),
                other_line: LineSegment(point![0.0, 0.0], point![42.0, 0.0]),
                expected_angle: FRAC_PI_4,
            },
            Case {
                self_line: LineSegment(
                    point![0.0, 0.0],
                    point![(-thirty_degree).cos(), (-thirty_degree).sin()],
                ),
                other_line: LineSegment(
                    point![0.0, 0.0],
                    point![thirty_degree.cos(), thirty_degree.sin()],
                ),
                expected_angle: sixty_degree,
            },
            Case {
                self_line: LineSegment(
                    point![0.0, 0.0],
                    point![thirty_degree.cos(), thirty_degree.sin()],
                ),
                other_line: LineSegment(
                    point![0.0, 0.0],
                    point![(-thirty_degree).cos(), (-thirty_degree).sin()],
                ),
                expected_angle: -sixty_degree,
            },
            Case {
                self_line: LineSegment(
                    point![0.0, 0.0],
                    point![(-sixty_degree).cos(), (-sixty_degree).sin()],
                ),
                other_line: LineSegment(
                    point![0.0, 0.0],
                    point![sixty_degree.cos(), sixty_degree.sin()],
                ),
                expected_angle: -sixty_degree,
            },
            Case {
                self_line: LineSegment(
                    point![0.0, 0.0],
                    point![sixty_degree.cos(), sixty_degree.sin()],
                ),
                other_line: LineSegment(
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
                    self_line: LineSegment(case.self_line.1, case.self_line.0),
                    other_line: case.other_line,
                    expected_angle: case.expected_angle,
                },
                Case {
                    self_line: case.self_line,
                    other_line: LineSegment(case.other_line.1, case.other_line.0),
                    expected_angle: case.expected_angle,
                },
                Case {
                    self_line: LineSegment(case.self_line.1, case.self_line.0),
                    other_line: LineSegment(case.other_line.1, case.other_line.0),
                    expected_angle: case.expected_angle,
                },
            ]
        });

        for case in cases {
            assert_relative_eq!(
                case.self_line.signed_acute_angle(case.other_line),
                case.expected_angle,
                epsilon = 0.000001,
            );
        }
    }

    #[test]
    fn arc_intersections() {
        let arc: Arc<SomeFrame> = Arc {
            circle: Circle {
                center: point![1.0, 1.0],
                radius: 1.0,
            },
            start: Angle(0.0),
            end: Angle(FRAC_PI_2),
            direction: Direction::Counterclockwise,
        };

        assert!(!LineSegment(point![0.0, 2.0], point![2.0, 0.0]).overlaps_arc(arc));
        assert!(!LineSegment(point![2.0, 2.0], point![3.0, 3.0]).overlaps_arc(arc));
        assert!(!LineSegment(point![0.0, 1.0], point![3.0, 0.0]).overlaps_arc(arc));
        assert!(LineSegment(point![0.0, 1.0], point![3.0, 2.0]).overlaps_arc(arc));
        assert!(LineSegment(point![0.0, 0.0], point![2.0, 2.0]).overlaps_arc(arc));

        let arc: Arc<SomeFrame> = Arc {
            circle: Circle {
                center: point![1.0, 1.0],
                radius: 1.0,
            },
            start: Angle(0.0),
            end: Angle(FRAC_PI_2),
            direction: Direction::Clockwise,
        };

        assert!(!LineSegment(point![1.0, 1.0], point![3.0, 3.0]).overlaps_arc(arc));
        assert!(!LineSegment(point![0.5, 1.0], point![1.5, 1.0]).overlaps_arc(arc));
        assert!(!LineSegment(point![0.0, 3.0], point![2.0, 3.0]).overlaps_arc(arc));
        assert!(LineSegment(point![0.0, 2.0], point![2.0, 0.0]).overlaps_arc(arc));
        assert!(LineSegment(point![0.0, 0.0], point![2.0, 2.0]).overlaps_arc(arc));
        assert!(LineSegment(point![-1.0, 1.0], point![2.0, 0.0]).overlaps_arc(arc));
    }
}
