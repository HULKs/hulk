use std::f32::consts::PI;

use approx::{AbsDiffEq, RelativeEq};
use nalgebra::{vector, Point2, Vector2};
use serde::{Deserialize, Serialize};

use crate::{arc::Arc, orientation::Orientation};

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct LineSegment(pub Point2<f32>, pub Point2<f32>);

impl AbsDiffEq for LineSegment {
    type Epsilon = f32;

    fn default_epsilon() -> Self::Epsilon {
        f32::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        Point2::abs_diff_eq(&other.0, &self.0, epsilon)
            && Point2::abs_diff_eq(&other.1, &self.1, epsilon)
    }
}

impl RelativeEq for LineSegment {
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
    pub fn new(start: Point2<f32>, end: Point2<f32>) -> Self {
        Self(start, end)
    }
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
            f => panic!("directed cathetus was not a real number: {f}"),
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
                angle_start_to_obstacle += TAU;
            }

            if angle_start_to_end < 0.0 {
                angle_start_to_end += TAU;
            }

            if (angle_start_to_obstacle < angle_start_to_end)
                ^ (orientation == Orientation::Clockwise)
            {
                return true;
            }
        }
        false
    }

    pub fn translate(&self, translation: &Vector2<f32>) -> Self {
        Self::new(self.0 + translation, self.1 + translation)
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;
    use nalgebra::point;

    use super::*;

    #[test]
    fn line_segment_lengths() {
        let line_segment = LineSegment(Point2::origin(), point!(0.0, 5.0));
        assert_relative_eq!(line_segment.norm(), 5.0);
        assert_relative_eq!(line_segment.norm_squared(), 5.0 * 5.0);
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
