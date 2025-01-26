use approx::{AbsDiffEq, RelativeEq};
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

use linear_algebra::{distance, vector, Point2};

use crate::{
    angle::Angle, arc::Arc, circle_tangents::CircleTangents, line_segment::LineSegment,
    rectangle::Rectangle, two_line_segments::TwoLineSegments, Distance,
};

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Deserialize,
    PartialEq,
    PathDeserialize,
    PathIntrospect,
    PathSerialize,
    Serialize,
)]
pub struct Circle<Frame> {
    pub center: Point2<Frame>,
    pub radius: f32,
}

impl<Frame> AbsDiffEq for Circle<Frame>
where
    Frame: AbsDiffEq,
{
    type Epsilon = f32;

    fn default_epsilon() -> Self::Epsilon {
        Self::Epsilon::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.center.abs_diff_eq(&other.center, epsilon)
            && self.radius.abs_diff_eq(&other.radius, epsilon)
    }
}

impl<Frame> RelativeEq for Circle<Frame>
where
    Frame: RelativeEq,
{
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

impl<Frame> Circle<Frame>
where
    Frame: Copy,
{
    pub fn new(center: Point2<Frame>, radius: f32) -> Self {
        Self { center, radius }
    }

    pub fn contains(&self, point: Point2<Frame>) -> bool {
        distance(self.center, point) <= self.radius
    }

    pub fn bounding_box(&self) -> Rectangle<Frame> {
        let radius_vector = vector![self.radius, self.radius];

        Rectangle {
            min: self.center - radius_vector,
            max: self.center + radius_vector,
        }
    }

    pub fn intersects_line_segment(&self, line_segment: &LineSegment<Frame>) -> bool {
        line_segment.distance_to(self.center) <= self.radius
    }

    pub fn overlaps_arc(&self, arc: Arc<Frame>) -> bool {
        let squared_distance = (arc.circle.center - self.center).norm_squared();
        if squared_distance > (self.radius + arc.circle.radius).powi(2) {
            return false;
        }

        let vector_to_obstacle = self.center - arc.circle.center;
        let angle_to_obstacle = Angle::from_direction(vector_to_obstacle);
        let angle_start_to_obstacle = arc.start.angle_to(angle_to_obstacle, arc.direction);
        let angle_start_to_end = arc.start.angle_to(arc.end, arc.direction);

        angle_start_to_obstacle.0 < angle_start_to_end.0
    }

    pub fn tangents_with_point(&self, other: Point2<Frame>) -> Option<TwoLineSegments<Frame>> {
        let delta_to_point = self.center - other;
        if delta_to_point.norm_squared() <= self.radius.powi(2) {
            return None;
        }

        let relative_tangent_angle = (self.radius / delta_to_point.norm()).asin();
        let angle_to_point = delta_to_point.y().atan2(delta_to_point.x());

        Some(TwoLineSegments(
            LineSegment(
                self.center
                    + vector![
                        (angle_to_point - relative_tangent_angle).sin(),
                        -(angle_to_point - relative_tangent_angle).cos()
                    ] * self.radius,
                other,
            ),
            LineSegment(
                self.center
                    + vector![
                        -(angle_to_point + relative_tangent_angle).sin(),
                        (angle_to_point + relative_tangent_angle).cos()
                    ] * self.radius,
                other,
            ),
        ))
    }

    fn interior_tangents_with_circle(
        &self,
        other: Circle<Frame>,
    ) -> Option<TwoLineSegments<Frame>> {
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
            let tangents = TwoLineSegments(
                LineSegment(reduced_tangents.0 .0 - shift1, small_circle.center - shift1),
                LineSegment(reduced_tangents.1 .0 - shift2, small_circle.center - shift2),
            );
            if flip {
                return Some(TwoLineSegments(tangents.0.flip(), tangents.1.flip()));
            }
            return Some(tangents);
        }

        None
    }

    fn exterior_tangents_with_circle(
        &self,
        other: Circle<Frame>,
    ) -> Option<TwoLineSegments<Frame>> {
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
            let tangents = TwoLineSegments(
                LineSegment(reduced_tangents.0 .0 + shift1, small_circle.center + shift1),
                LineSegment(reduced_tangents.1 .0 + shift2, small_circle.center + shift2),
            );
            if flip {
                return Some(TwoLineSegments(tangents.0.flip(), tangents.1.flip()));
            }
            return Some(tangents);
        }

        None
    }

    pub fn tangents_with_circle(&self, other: Circle<Frame>) -> Option<CircleTangents<Frame>> {
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

    pub fn point_at_angle(&self, angle: Angle<f32>) -> Point2<Frame> {
        self.center + angle.as_direction() * self.radius
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;
    use linear_algebra::point;

    use super::*;

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct SomeFrame;

    #[test]
    fn circle_line_intersection() {
        let circle = Circle::<SomeFrame>::new(point![0.0, 0.0], 1.0);
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
        let circle = Circle::<SomeFrame>::new(point![0.0, 0.0], 2.0_f32.sqrt() / 2.0);
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
            CircleTangents::<SomeFrame> {
                inner: Some(TwoLineSegments(
                    LineSegment(point_left, point_right),
                    LineSegment(point_left, point_right)
                )),
                outer: TwoLineSegments(
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
            CircleTangents::<SomeFrame> {
                inner: Some(TwoLineSegments(
                    LineSegment(point![-0.5, 0.5], point_right),
                    LineSegment(point![-0.5, -0.5], point_right)
                )),
                outer: TwoLineSegments(
                    LineSegment(point![-0.5, 0.5], point_right),
                    LineSegment(point![-0.5, -0.5], point_right)
                )
            },
            epsilon = 0.001
        )
    }

    #[test]
    fn no_tangents_for_enclosing_circles() {
        let small_circle = Circle::<SomeFrame>::new(point![0.0, 0.0], 1.0);
        let large_circle = Circle::<SomeFrame>::new(point![0.0, 0.0], 2.0);

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
            CircleTangents::<SomeFrame> {
                inner: None,
                outer: TwoLineSegments(
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
            CircleTangents::<SomeFrame> {
                inner: None,
                outer: TwoLineSegments(
                    LineSegment(point![-0.5, 1.0], point![0.5, 1.0]),
                    LineSegment(point![-0.5, -1.0], point![0.5, -1.0]),
                )
            },
            epsilon = 0.001
        )
    }
}
