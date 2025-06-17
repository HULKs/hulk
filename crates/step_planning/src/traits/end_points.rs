use coordinate_systems::Ground;
use geometry::{arc::Arc, line_segment::LineSegment};
use linear_algebra::Point2;
use types::planned_path::PathSegment;

pub trait EndPoints<Frame> {
    fn start_point(&self) -> Point2<Frame>;
    fn end_point(&self) -> Point2<Frame>;
}

impl<Frame: Copy> EndPoints<Frame> for Arc<Frame> {
    fn start_point(&self) -> Point2<Frame> {
        self.circle.point_at_angle(self.start)
    }

    fn end_point(&self) -> Point2<Frame> {
        self.circle.point_at_angle(self.end)
    }
}

impl<Frame> EndPoints<Frame> for LineSegment<Frame> {
    fn start_point(&self) -> Point2<Frame> {
        self.0
    }

    fn end_point(&self) -> Point2<Frame> {
        self.1
    }
}

impl EndPoints<Ground> for PathSegment {
    fn start_point(&self) -> Point2<Ground> {
        match self {
            PathSegment::LineSegment(line_segment) => line_segment.start_point(),
            PathSegment::Arc(arc) => arc.start_point(),
        }
    }

    fn end_point(&self) -> Point2<Ground> {
        match self {
            PathSegment::LineSegment(line_segment) => line_segment.end_point(),
            PathSegment::Arc(arc) => arc.end_point(),
        }
    }
}
