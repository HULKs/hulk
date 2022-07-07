use approx::{AbsDiffEq, RelativeEq};
use nalgebra::{vector, Point2, UnitComplex, Vector2};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

use std::f32::consts::PI;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Orientation {
    Clockwise,
    Counterclockwise,
    Colinear,
}

impl Orientation {
    pub fn rotate_vector_90_degrees(&self, subject: Vector2<f32>) -> Vector2<f32> {
        match self {
            Orientation::Clockwise => vector![subject.y, -subject.x],
            Orientation::Counterclockwise => vector![-subject.y, subject.x],
            Orientation::Colinear => subject,
        }
    }
}

pub fn rotate_towards(origin: Point2<f32>, target: Point2<f32>) -> UnitComplex<f32> {
    let origin_to_target = target - origin;
    UnitComplex::rotation_between(&Vector2::x(), &origin_to_target)
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct LineSegment(pub Point2<f32>, pub Point2<f32>);

impl approx::AbsDiffEq for LineSegment {
    type Epsilon = f32;

    fn default_epsilon() -> Self::Epsilon {
        f32::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        Point2::abs_diff_eq(&other.0, &self.0, epsilon)
            && Point2::abs_diff_eq(&other.1, &self.1, epsilon)
    }
}

impl approx::RelativeEq for LineSegment {
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

impl LineSegment {
    pub fn flip(self) -> Self {
        Self(self.1, self.0)
    }

    pub fn norm(&self) -> f32 {
        (self.0 - self.1).norm()
    }

    pub fn norm_squared(&self) -> f32 {
        (self.0 - self.1).norm_squared()
    }

    pub fn projection_factor(&self, point: Point2<f32>) -> f32 {
        let projection = (point - self.0).dot(&(self.1 - self.0));

        projection / self.norm_squared()
    }

    pub fn closest_point(&self, point: Point2<f32>) -> Point2<f32> {
        let projected_factor = self.projection_factor(point).clamp(0.0, 1.0);
        self.0 + projected_factor * (self.1 - self.0)
    }

    pub fn shortest_distance_to_point(&self, other_point: Point2<f32>) -> f32 {
        (other_point - self.closest_point(other_point)).norm()
    }

    /// Reference: https://algotree.org/algorithms/computational_geometry/line_segment_intersection/
    pub fn intersects_line_segment(&self, other: LineSegment) -> bool {
        let orientation_other_points_to_self =
            (self.get_orientation(other.0), self.get_orientation(other.1));

        match orientation_other_points_to_self {
            (Orientation::Counterclockwise, Orientation::Counterclockwise)
            | (Orientation::Clockwise, Orientation::Clockwise) => false,

            (Orientation::Colinear, Orientation::Colinear) => {
                self.overlaps_collinear_line_segment(other)
            }

            _ => {
                let orientation_self_points_to_other =
                    (other.get_orientation(self.0), other.get_orientation(self.1));

                orientation_self_points_to_other.0 != orientation_self_points_to_other.1
                    || orientation_self_points_to_other.0 == Orientation::Colinear
            }
        }
    }

    fn overlaps_collinear_line_segment(&self, other: LineSegment) -> bool {
        self.bounding_box_contains(other.0)
            || self.bounding_box_contains(other.1)
            || other.bounding_box_contains(self.0)
            || other.bounding_box_contains(self.1)
    }

    fn bounding_box_contains(&self, point: Point2<f32>) -> bool {
        point.x > f32::min(self.0.x, self.1.x)
            && point.x < f32::max(self.0.x, self.1.x)
            && point.y < f32::max(self.0.y, self.1.y)
            && point.y > f32::min(self.0.y, self.1.y)
    }

    pub fn get_orientation(&self, point: Point2<f32>) -> Orientation {
        let direction_vector = self.1 - self.0;
        let clockwise_normal_vector = vector![direction_vector.y, -direction_vector.x];
        let directed_cathetus = clockwise_normal_vector.dot(&(point - self.0));

        match directed_cathetus {
            f if f == 0.0 => Orientation::Colinear,
            f if f > 0.0 => Orientation::Clockwise,
            f if f < 0.0 => Orientation::Counterclockwise,
            f => panic!("Directed cathetus was not a real number: {}", f),
        }
    }

    pub fn overlaps_arc(&self, arc: Arc, orientation: Orientation) -> bool {
        if self.shortest_distance_to_point(arc.circle.center) >= arc.circle.radius {
            return false;
        }

        let projection = (arc.circle.center - self.0).dot(&(self.1 - self.0));
        let projected_point_relative_contribution = projection / self.norm_squared();
        let base_point = self.0 + projected_point_relative_contribution * (self.1 - self.0);

        let center_to_base_length = (base_point - arc.circle.center).norm();
        let base_to_intersection_length =
            f32::sqrt(arc.circle.radius.powi(2) - center_to_base_length.powi(2));

        let direction_vector = vector![self.1.x - self.0.x, self.1.y - self.0.y];
        let normed_direction_vector = direction_vector.normalize();

        let intersection_point1 =
            base_point + base_to_intersection_length * normed_direction_vector;
        let intersection_point2 =
            base_point - base_to_intersection_length * normed_direction_vector;

        let mut intersection_points: Vec<Point2<f32>> = Vec::new();
        if (0.0..1.0).contains(&self.projection_factor(intersection_point1)) {
            intersection_points.push(intersection_point1)
        }
        if (0.0..1.0).contains(&self.projection_factor(intersection_point2)) {
            intersection_points.push(intersection_point2)
        }
        let vector_start = arc.start - arc.circle.center;
        let vector_end = arc.end - arc.circle.center;

        let angle_x_axis_to_start = vector_start.y.atan2(vector_start.x);
        let mut angle_start_to_end = vector_end.y.atan2(vector_end.x) - angle_x_axis_to_start;

        for intersection_point in &intersection_points {
            let vector_obstacle = intersection_point - arc.circle.center;
            let mut angle_start_to_obstacle =
                vector_obstacle.y.atan2(vector_obstacle.x) - angle_x_axis_to_start;

            if angle_start_to_obstacle < 0.0 {
                angle_start_to_obstacle += 2.0 * PI;
            }

            if angle_start_to_end < 0.0 {
                angle_start_to_end += 2.0 * PI;
            }

            if (angle_start_to_obstacle < angle_start_to_end)
                ^ (orientation == Orientation::Clockwise)
            {
                return true;
            }
        }
        false
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize, SerializeHierarchy)]
pub struct CircleTangents {
    #[leaf]
    pub inner: Option<(LineSegment, LineSegment)>,
    #[leaf]
    pub outer: (LineSegment, LineSegment),
}

impl approx::AbsDiffEq for CircleTangents {
    type Epsilon = f32;

    fn default_epsilon() -> Self::Epsilon {
        f32::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.inner.is_some() == other.inner.is_some()
            && LineSegment::abs_diff_eq(&other.outer.0, &self.outer.0, epsilon)
            && LineSegment::abs_diff_eq(&other.outer.1, &self.outer.1, epsilon)
            && if self.inner.is_some() && other.inner.is_some() {
                LineSegment::abs_diff_eq(&other.inner.unwrap().0, &self.inner.unwrap().0, epsilon)
                    && LineSegment::abs_diff_eq(
                        &other.inner.unwrap().1,
                        &self.inner.unwrap().1,
                        epsilon,
                    )
            } else {
                true
            }
    }
}

impl approx::RelativeEq for CircleTangents {
    fn default_max_relative() -> f32 {
        f32::default_max_relative()
    }

    fn relative_eq(
        &self,
        other: &Self,
        epsilon: Self::Epsilon,
        max_relative: Self::Epsilon,
    ) -> bool {
        self.inner.is_some() == other.inner.is_some()
            && LineSegment::relative_eq(&other.outer.0, &self.outer.0, epsilon, max_relative)
            && LineSegment::relative_eq(&other.outer.1, &self.outer.1, epsilon, max_relative)
            && if self.inner.is_some() && other.inner.is_some() {
                LineSegment::relative_eq(
                    &other.inner.unwrap().0,
                    &self.inner.unwrap().0,
                    epsilon,
                    max_relative,
                ) && LineSegment::relative_eq(
                    &other.inner.unwrap().1,
                    &self.inner.unwrap().1,
                    epsilon,
                    max_relative,
                )
            } else {
                true
            }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize, SerializeHierarchy)]
pub struct Arc {
    pub circle: Circle,
    pub start: Point2<f32>,
    pub end: Point2<f32>,
}

impl AbsDiffEq for Arc {
    type Epsilon = f32;

    fn default_epsilon() -> Self::Epsilon {
        Self::Epsilon::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.circle.abs_diff_eq(&other.circle, epsilon)
            && self.start.abs_diff_eq(&other.start, epsilon)
            && self.end.abs_diff_eq(&other.end, epsilon)
    }
}

impl RelativeEq for Arc {
    fn default_max_relative() -> f32 {
        f32::default_max_relative()
    }

    fn relative_eq(
        &self,
        other: &Self,
        epsilon: Self::Epsilon,
        max_relative: Self::Epsilon,
    ) -> bool {
        self.circle
            .relative_eq(&other.circle, epsilon, max_relative)
            && self.start.relative_eq(&other.start, epsilon, max_relative)
            && self.end.relative_eq(&other.end, epsilon, max_relative)
    }
}

impl Arc {
    pub fn new(circle: Circle, start: Point2<f32>, end: Point2<f32>) -> Self {
        Self { circle, start, end }
    }

    pub fn length(&self, orientation: Orientation) -> f32 {
        let vector_start = self.start - self.circle.center;
        let vector_end = self.end - self.circle.center;

        let angle_x_axis_to_start = vector_start.y.atan2(vector_start.x);
        let mut angle = vector_end.y.atan2(vector_end.x) - angle_x_axis_to_start;

        if (orientation == Orientation::Clockwise) && (angle > 0.0) {
            angle -= 2.0 * PI;
            angle *= -1.0;
        }
        if (orientation == Orientation::Counterclockwise) && (angle < 0.0) {
            angle += 2.0 * PI;
        }
        (angle * self.circle.radius).abs()
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize, SerializeHierarchy)]
pub struct Circle {
    pub center: Point2<f32>,
    pub radius: f32,
}

impl AbsDiffEq for Circle {
    type Epsilon = f32;

    fn default_epsilon() -> Self::Epsilon {
        Self::Epsilon::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.center.abs_diff_eq(&other.center, epsilon)
            && self.radius.abs_diff_eq(&other.radius, epsilon)
    }
}

impl RelativeEq for Circle {
    fn default_max_relative() -> Self::Epsilon {
        Self::Epsilon::default_max_relative()
    }

    fn relative_eq(
        &self,
        other: &Self,
        epsilon: Self::Epsilon,
        max_relative: Self::Epsilon,
    ) -> bool {
        self.center
            .relative_eq(&other.center, epsilon, max_relative)
            && self
                .radius
                .relative_eq(&other.radius, epsilon, max_relative)
    }
}

impl Circle {
    pub fn new(center: Point2<f32>, radius: f32) -> Self {
        Self { center, radius }
    }

    pub fn bounding_box(&self) -> Rectangle {
        let radius_vector = vector![self.radius, self.radius];

        Rectangle {
            top_left: self.center - radius_vector,
            bottom_right: self.center + radius_vector,
        }
    }

    pub fn intersects_line_segment(&self, line_segment: &LineSegment) -> bool {
        line_segment.shortest_distance_to_point(self.center) <= self.radius
    }

    pub fn overlaps_arc(&self, arc: Arc, orientation: Orientation) -> bool {
        let distance = (arc.circle.center - self.center).norm_squared();
        if distance > (self.radius + arc.circle.radius).powi(2) {
            return false;
        }

        let vector_start = arc.start - arc.circle.center;
        let vector_obstacle = self.center - arc.circle.center;
        let vector_end = arc.end - arc.circle.center;

        let angle_x_axis_to_start = vector_start.y.atan2(vector_start.x);
        let mut angle_start_to_obstacle =
            vector_obstacle.y.atan2(vector_obstacle.x) - angle_x_axis_to_start;

        let mut angle_start_to_end = vector_end.y.atan2(vector_end.x) - angle_x_axis_to_start;

        if angle_start_to_obstacle < 0.0 {
            angle_start_to_obstacle += 2.0 * PI;
        }

        if angle_start_to_end < 0.0 {
            angle_start_to_end += 2.0 * PI;
        }

        (angle_start_to_obstacle < angle_start_to_end) ^ (orientation == Orientation::Clockwise)
    }

    pub fn tangents_with_point(&self, other: Point2<f32>) -> Option<(LineSegment, LineSegment)> {
        let delta_to_point = self.center - other;
        if delta_to_point.norm_squared() <= self.radius.powi(2) {
            return None;
        }

        let relative_tangent_angle = (self.radius / delta_to_point.norm()).asin();
        let angle_to_point = delta_to_point.y.atan2(delta_to_point.x);

        Some((
            LineSegment(
                self.center
                    + self.radius
                        * vector![
                            (angle_to_point - relative_tangent_angle).sin(),
                            -(angle_to_point - relative_tangent_angle).cos()
                        ],
                other,
            ),
            LineSegment(
                self.center
                    + self.radius
                        * vector![
                            -(angle_to_point + relative_tangent_angle).sin(),
                            (angle_to_point + relative_tangent_angle).cos()
                        ],
                other,
            ),
        ))
    }

    fn interior_tangents_with_circle(&self, other: Circle) -> Option<(LineSegment, LineSegment)> {
        let flip = other.radius > self.radius;
        let small_circle = if flip { self } else { &other };
        let large_circle = if flip { &other } else { self };

        let reduced_circle = Circle::new(
            large_circle.center,
            large_circle.radius + small_circle.radius + f32::EPSILON,
        );
        let radius_change_ratio = small_circle.radius / reduced_circle.radius;
        if let Some(reduced_tangents) = reduced_circle.tangents_with_point(small_circle.center) {
            let shift1 = (reduced_tangents.0 .0 - large_circle.center) * radius_change_ratio;
            let shift2 = (reduced_tangents.1 .0 - large_circle.center) * radius_change_ratio;
            let tangents = (
                LineSegment(reduced_tangents.0 .0 - shift1, small_circle.center - shift1),
                LineSegment(reduced_tangents.1 .0 - shift2, small_circle.center - shift2),
            );
            if flip {
                return Some((tangents.0.flip(), tangents.1.flip()));
            }
            return Some(tangents);
        }

        None
    }

    fn exterior_tangents_with_circle(&self, other: Circle) -> Option<(LineSegment, LineSegment)> {
        let flip = other.radius > self.radius;
        let small_circle = if flip { self } else { &other };
        let large_circle = if flip { &other } else { self };

        let reduced_circle = Circle::new(
            large_circle.center,
            large_circle.radius - small_circle.radius + f32::EPSILON,
        );
        let radius_change_ratio = small_circle.radius / reduced_circle.radius;
        if let Some(reduced_tangents) = reduced_circle.tangents_with_point(small_circle.center) {
            let shift1 = (reduced_tangents.0 .0 - large_circle.center) * radius_change_ratio;
            let shift2 = (reduced_tangents.1 .0 - large_circle.center) * radius_change_ratio;
            let tangents = (
                LineSegment(reduced_tangents.0 .0 + shift1, small_circle.center + shift1),
                LineSegment(reduced_tangents.1 .0 + shift2, small_circle.center + shift2),
            );
            if flip {
                return Some((tangents.0.flip(), tangents.1.flip()));
            }
            return Some(tangents);
        }

        None
    }

    pub fn tangents_with_circle(&self, other: Circle) -> Option<CircleTangents> {
        let squared_distance = (self.center - other.center).norm_squared();

        let enclosure_radius =
            f32::max(self.radius, other.radius) - f32::min(self.radius, other.radius);
        if squared_distance <= enclosure_radius.powi(2) {
            return None;
        }

        let touch_radius = self.radius + other.radius;
        let inner = if squared_distance > touch_radius.powi(2) {
            self.interior_tangents_with_circle(other)
        } else {
            None
        };

        let outer = self.exterior_tangents_with_circle(other)?;

        Some(CircleTangents { inner, outer })
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize, SerializeHierarchy)]
pub struct Rectangle {
    pub top_left: Point2<f32>,
    pub bottom_right: Point2<f32>,
}

impl Rectangle {
    pub fn rectangle_intersection(self, other: Rectangle) -> f32 {
        let intersection_x = f32::max(
            0.0,
            f32::min(self.bottom_right.x, other.bottom_right.x)
                - f32::max(self.top_left.x, other.top_left.x),
        );
        let intersection_y = f32::max(
            0.0,
            f32::min(self.bottom_right.y, other.bottom_right.y)
                - f32::max(self.top_left.y, other.top_left.y),
        );
        intersection_x * intersection_y
    }

    pub fn area(self) -> f32 {
        let dimensions = self.bottom_right - self.top_left;
        dimensions.x * dimensions.y
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::PI;

    use approx::{assert_relative_eq, assert_relative_ne};
    use nalgebra::{point, Point2, UnitComplex};

    use super::*;

    #[test]
    fn arc_cost_90_degrees() {
        let arc = Arc {
            circle: Circle {
                center: point![1.0, 1.0],
                radius: 2.0,
            },
            start: point![1.0, 2.0],
            end: point![2.0, 1.0],
        };
        assert_relative_eq!(arc.length(Orientation::Clockwise), PI);
        assert_relative_eq!(arc.length(Orientation::Counterclockwise), 3.0 * PI);

        let arc = Arc {
            circle: Circle {
                center: point![1.0, 1.0],
                radius: 2.0,
            },
            start: point![2.0, 1.0],
            end: point![1.0, 2.0],
        };
        assert_relative_eq!(arc.length(Orientation::Clockwise), 3.0 * PI);
        assert_relative_eq!(arc.length(Orientation::Counterclockwise), PI);
    }

    #[test]
    fn arc_cost_generic() {
        for angle_index in 0..100 {
            let angle = angle_index as f32 / 100.0 * 2.0 * PI;
            for angle_distance_index in 1..100 {
                let angle_distance = angle_distance_index as f32 / 100.0 * 2.0 * PI;
                let start = UnitComplex::from_angle(angle) * vector![1.0, 0.0];
                let end = UnitComplex::from_angle(angle + angle_distance) * vector![1.0, 0.0];
                let center = point![3.14, 4.20];
                let radius = 5.0;
                let arc = Arc {
                    circle: Circle { center, radius },
                    start: center + start,
                    end: center + end,
                };

                println!("angle: {} angle_distance {}", angle, angle_distance);
                assert_relative_eq!(
                    arc.length(Orientation::Counterclockwise),
                    radius * angle_distance,
                    epsilon = 0.001
                );
                assert_relative_eq!(
                    arc.length(Orientation::Clockwise),
                    radius * (2.0 * PI - angle_distance),
                    epsilon = 0.001
                );
            }
        }
    }

    #[test]
    fn circle_cmp_same() {
        assert_relative_eq!(
            Circle {
                center: point![1337.5, 42.5],
                radius: f32::sqrt(2.0),
            },
            Circle {
                center: point![1337.5, 42.5],
                radius: f32::sin(PI / 4.0) * 2.0,
            },
        );
    }

    #[test]
    fn circle_cmp_different_radius() {
        assert_relative_ne!(
            Circle {
                center: point![1337.5, 42.5],
                radius: f32::sqrt(3.0),
            },
            Circle {
                center: point![1337.5, 42.5],
                radius: f32::sin(PI / 4.0) * 2.0,
            },
        );
    }

    #[test]
    fn circle_cmp_different_center() {
        assert_relative_ne!(
            Circle {
                center: point![1337.1, 42.5],
                radius: f32::sqrt(2.0),
            },
            Circle {
                center: point![1337.5, 52.5],
                radius: f32::sin(PI / 4.0) * 2.0,
            },
        );
    }

    #[test]
    fn line_segment_lengths() {
        for i in 0..10 {
            let line_segment = LineSegment(Point2::origin(), point!(0.0, i as f32));
            assert_relative_eq!(line_segment.norm(), i as f32);
            assert_relative_eq!(line_segment.norm_squared(), (i * i) as f32);
        }
        let diagonal = LineSegment(point![-1.0, -1.0], point![1.0, 1.0]);
        assert_relative_eq!(diagonal.norm(), 8.0_f32.sqrt());
        assert_relative_eq!(diagonal.norm_squared(), 8.0);
    }

    #[test]
    fn shortest_distance_between_point_and_line_segment() {
        let line_segment = LineSegment(point![-1.0, 0.0], point![1.0, 0.0]);

        assert_relative_eq!(
            0.0,
            line_segment.shortest_distance_to_point(point![-1.0, 0.0])
        );
        assert_relative_eq!(
            0.0,
            line_segment.shortest_distance_to_point(point![1.0, 0.0])
        );
        assert_relative_eq!(
            1.0,
            line_segment.shortest_distance_to_point(point![0.0, 1.0])
        );
        assert_relative_eq!(
            2.0_f32.sqrt(),
            line_segment.shortest_distance_to_point(point![2.0, -1.0])
        );
        assert_relative_eq!(
            0.5,
            line_segment.shortest_distance_to_point(point![-0.5, -0.5])
        );
    }

    #[test]
    fn circle_line_intersection() {
        let circle = Circle::new(point![0.0, 0.0], 1.0);
        let fully_outside = LineSegment(point![2.0, 0.0], point![0.0, 2.0]);
        let middle_intersection = LineSegment(point![-1.0, -1.0], point![1.0, 0.5]);
        let p1_interior = LineSegment(point![0.5, 0.5], point![5.0, 1.5]);
        let p2_interior = LineSegment(point![55.0, 42.123], point![0.25, 0.3]);
        let fully_enclosed = LineSegment(point![-0.5, -0.5], point![0.5, 0.5]);

        assert!(!circle.intersects_line_segment(&fully_outside));
        assert!(circle.intersects_line_segment(&middle_intersection));
        assert!(circle.intersects_line_segment(&p1_interior));
        assert!(circle.intersects_line_segment(&p2_interior));
        assert!(circle.intersects_line_segment(&fully_enclosed));
    }

    #[test]
    fn tangents_between_circle_and_point() {
        let circle = Circle::new(point![0.0, 0.0], 2.0_f32.sqrt() / 2.0);
        let point = point![1.0, 0.0];

        let tangents = circle
            .tangents_with_point(point)
            .expect("Could not generate tangents");

        assert_relative_eq!(
            tangents.0,
            LineSegment(point![0.5, 0.5], point),
            epsilon = 0.001
        );
        assert_relative_eq!(
            tangents.1,
            LineSegment(point![0.5, -0.5], point),
            epsilon = 0.001
        );
    }

    #[test]
    fn tangents_between_degenerate_circles() {
        let point_left = point![-1.0, 0.0];
        let point_right = point![1.0, 0.0];
        let circle_left = Circle::new(point_left, 0.0);
        let circle_right = Circle::new(point_right, 0.0);

        let tangents = circle_left
            .tangents_with_circle(circle_right)
            .expect("Could not generate tangents");

        assert_relative_eq!(
            tangents,
            CircleTangents {
                inner: Some((
                    LineSegment(point_left, point_right),
                    LineSegment(point_left, point_right)
                )),
                outer: (
                    LineSegment(point_left, point_right),
                    LineSegment(point_left, point_right)
                )
            },
            epsilon = 0.001
        );
    }

    #[test]
    fn tangents_with_one_degenerate_circle() {
        let point_left = point![-1.0, 0.0];
        let point_right = point![0.0, 0.0];
        let circle_left = Circle::new(point_left, 2.0_f32.sqrt() / 2.0);
        let circle_right = Circle::new(point_right, 0.0);

        let tangents = circle_left
            .tangents_with_circle(circle_right)
            .expect("Could not generate tangents");

        assert_relative_eq!(
            tangents,
            CircleTangents {
                inner: Some((
                    LineSegment(point![-0.5, 0.5], point_right),
                    LineSegment(point![-0.5, -0.5], point_right)
                )),
                outer: (
                    LineSegment(point![-0.5, 0.5], point_right),
                    LineSegment(point![-0.5, -0.5], point_right)
                )
            },
            epsilon = 0.001
        )
    }

    #[test]
    fn no_tangents_for_enclosing_circles() {
        let small_circle = Circle::new(point![0.0, 0.0], 1.0);
        let large_circle = Circle::new(point![0.0, 0.0], 2.0);

        assert_eq!(small_circle.tangents_with_circle(large_circle), None);
        assert_eq!(large_circle.tangents_with_circle(small_circle), None);
    }

    #[test]
    fn tangents_with_touching_circles() {
        let point_left = point![-0.5, 0.0];
        let point_right = point![0.5, 0.0];
        let circle_left = Circle::new(point_left, 1.0);
        let circle_right = Circle::new(point_right, 1.0);

        let tangents = circle_left
            .tangents_with_circle(circle_right)
            .expect("Could not generate tangents");

        assert_relative_eq!(
            tangents,
            CircleTangents {
                inner: None,
                outer: (
                    LineSegment(point![-0.5, 1.0], point![0.5, 1.0]),
                    LineSegment(point![-0.5, -1.0], point![0.5, -1.0]),
                )
            },
            epsilon = 0.001
        )
    }

    #[test]
    fn tangents_with_disconnected_circles() {
        let point_left = point![-0.5, 0.0];
        let point_right = point![0.5, 0.0];
        let circle_left = Circle::new(point_left, 1.0);
        let circle_right = Circle::new(point_right, 1.0);

        let tangents = circle_left
            .tangents_with_circle(circle_right)
            .expect("Could not generate tangents");

        assert_relative_eq!(
            tangents,
            CircleTangents {
                inner: None,
                outer: (
                    LineSegment(point![-0.5, 1.0], point![0.5, 1.0]),
                    LineSegment(point![-0.5, -1.0], point![0.5, -1.0]),
                )
            },
            epsilon = 0.001
        )
    }

    fn test_all_permutations(
        reference_line_segment: LineSegment,
        line_segment: LineSegment,
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
}
