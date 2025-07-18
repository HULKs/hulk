use coordinate_systems::Ground;
use geometry::{arc::Arc, direction::Rotate90Degrees, line_segment::LineSegment};
use types::planned_path::{Path, PathSegment};

use crate::geometry::orientation::Orientation;

pub trait ForwardAtEndPoint {
    fn forward_at_end_point(&self) -> Orientation<f32>;
}

impl ForwardAtEndPoint for Arc<Ground> {
    fn forward_at_end_point(&self) -> Orientation<f32> {
        Orientation::from_direction(self.end.as_unit_vector().rotate_90_degrees(self.direction))
    }
}

impl ForwardAtEndPoint for LineSegment<Ground> {
    fn forward_at_end_point(&self) -> Orientation<f32> {
        Orientation::from_direction(self.1 - self.0)
    }
}

impl ForwardAtEndPoint for PathSegment {
    fn forward_at_end_point(&self) -> Orientation<f32> {
        match self {
            PathSegment::LineSegment(line_segment) => line_segment.forward_at_end_point(),
            PathSegment::Arc(arc) => arc.forward_at_end_point(),
        }
    }
}

impl ForwardAtEndPoint for Path {
    fn forward_at_end_point(&self) -> Orientation<f32> {
        self.segments
            .last()
            .expect("path was empty")
            .forward_at_end_point()
    }
}
