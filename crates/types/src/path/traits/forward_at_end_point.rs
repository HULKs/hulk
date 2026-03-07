use coordinate_systems::Ground;
use geometry::{arc::Arc, direction::Rotate90Degrees, line_segment::LineSegment};
use linear_algebra::Orientation2;

use crate::path::{Path, PathSegment};

pub trait ForwardAtEndPoint {
    fn forward_at_end_point(&self) -> Orientation2<Ground>;
}

impl ForwardAtEndPoint for Arc<Ground> {
    fn forward_at_end_point(&self) -> Orientation2<Ground> {
        Orientation2::from_vector(self.end.as_unit_vector().rotate_90_degrees(self.direction))
    }
}

impl ForwardAtEndPoint for LineSegment<Ground> {
    fn forward_at_end_point(&self) -> Orientation2<Ground> {
        Orientation2::from_vector(self.1 - self.0)
    }
}

impl ForwardAtEndPoint for PathSegment {
    fn forward_at_end_point(&self) -> Orientation2<Ground> {
        match self {
            PathSegment::LineSegment(line_segment) => line_segment.forward_at_end_point(),
            PathSegment::Arc(arc) => arc.forward_at_end_point(),
        }
    }
}

impl ForwardAtEndPoint for Path {
    fn forward_at_end_point(&self) -> Orientation2<Ground> {
        self.last_segment().forward_at_end_point()
    }
}
