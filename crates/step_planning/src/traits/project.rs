use coordinate_systems::Ground;
use geometry::{arc::Arc, line_segment::LineSegment};
use linear_algebra::Point2;
use types::planned_path::{Path, PathSegment};

use crate::traits::{ArcProjectionKind, ClassifyProjection};

pub trait Project<Frame> {
    /// Project `point` onto `self`.
    /// In other words, find the point closest to `point` in `self`
    fn project(&self, point: Point2<Frame>) -> Point2<Frame>;
}

impl Project<Ground> for Path<'_> {
    fn project(&self, point: Point2<Ground>) -> Point2<Ground> {
        let (projected_point, _distance) = self
            .segments
            .iter()
            .map(|segment| {
                let projection = segment.project(point);
                let squared_distance = (projection - point).norm_squared();

                (projection, squared_distance)
            })
            .min_by(|(_, distance_1), (_, distance_2)| distance_1.total_cmp(distance_2))
            .expect("Path was empty");

        projected_point
    }
}

impl Project<Ground> for PathSegment {
    fn project(&self, point: Point2<Ground>) -> Point2<Ground> {
        match self {
            PathSegment::LineSegment(line_segment) => line_segment.project(point),
            PathSegment::Arc(arc) => arc.project(point),
        }
    }
}

impl Project<Ground> for LineSegment<Ground> {
    fn project(&self, point: Point2<Ground>) -> Point2<Ground> {
        let direction = self.1 - self.0;
        let v = point - self.0;
        let t = v.dot(&direction) / direction.inner.magnitude_squared().max(1e-5);

        self.0 + direction * t.clamp(0.0, 1.0)
    }
}

impl Project<Ground> for Arc<Ground> {
    fn project(&self, point: Point2<Ground>) -> Point2<Ground> {
        match self.classify_point(point) {
            ArcProjectionKind::OnArc => {
                let center_to_point = point - self.circle.center;

                self.circle.center + center_to_point.normalize() * self.circle.radius
            }
            ArcProjectionKind::Start => self.circle.point_at_angle(self.start),
            ArcProjectionKind::End => self.circle.point_at_angle(self.end),
        }
    }
}
