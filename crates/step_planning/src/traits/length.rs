use geometry::{arc::Arc, direction::AngleTo, line_segment::LineSegment};
use types::planned_path::{Path, PathSegment};

pub trait Length {
    fn length(&self) -> f32;
}

impl Length for Path {
    fn length(&self) -> f32 {
        self.segments.iter().map(PathSegment::length).sum()
    }
}

impl Length for PathSegment {
    fn length(&self) -> f32 {
        match self {
            PathSegment::LineSegment(line_segment) => line_segment.length(),
            PathSegment::Arc(arc) => arc.length(),
        }
    }
}

impl<Frame> Length for LineSegment<Frame> {
    fn length(&self) -> f32 {
        let Self(start, end) = self;

        (*end - *start).norm()
    }
}

impl<Frame> Length for Arc<Frame> {
    fn length(&self) -> f32 {
        let angle_start_to_end = self.start.angle_to(self.end, self.direction);

        self.circle.radius * angle_start_to_end
    }
}
