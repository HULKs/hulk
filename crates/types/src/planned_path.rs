use approx::{AbsDiffEq, RelativeEq};
use serde::{Deserialize, Serialize};

use geometry::{arc::Arc, line_segment::LineSegment};
use linear_algebra::Point2;
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};

#[derive(Clone, Debug, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect)]
pub enum PathSegment<Frame> {
    LineSegment(LineSegment<Frame>),
    Arc(Arc<Frame>),
}

impl<Frame> PartialEq for PathSegment<Frame>
where
    LineSegment<Frame>: PartialEq,
    Arc<Frame>: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::LineSegment(l0), Self::LineSegment(r0)) => l0 == r0,
            (Self::Arc(l0), Self::Arc(r0)) => l0 == r0,
            _ => false,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Path<'a, Frame> {
    pub segments: &'a [PathSegment<Frame>],
}

pub fn direct_path<Frame>(
    start: Point2<Frame>,
    destination: Point2<Frame>,
) -> Vec<PathSegment<Frame>> {
    vec![PathSegment::LineSegment(LineSegment(start, destination))]
}

impl<Frame: PartialEq> AbsDiffEq for PathSegment<Frame> {
    type Epsilon = f32;

    fn default_epsilon() -> Self::Epsilon {
        Self::Epsilon::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        match (self, other) {
            (
                PathSegment::LineSegment(line_segment_self),
                PathSegment::LineSegment(line_segment_other),
            ) => line_segment_self.abs_diff_eq(line_segment_other, epsilon),
            (PathSegment::Arc(arc_self), PathSegment::Arc(arc_other)) => {
                arc_self.abs_diff_eq(arc_other, epsilon)
            }
            _ => false,
        }
    }
}

impl<Frame: PartialEq> RelativeEq for PathSegment<Frame> {
    fn default_max_relative() -> f32 {
        f32::default_max_relative()
    }

    fn relative_eq(
        &self,
        other: &Self,
        epsilon: Self::Epsilon,
        max_relative: Self::Epsilon,
    ) -> bool {
        match (self, other) {
            (
                PathSegment::LineSegment(line_segment_self),
                PathSegment::LineSegment(line_segment_other),
            ) => line_segment_self.relative_eq(line_segment_other, epsilon, max_relative),
            (PathSegment::Arc(arc_self), PathSegment::Arc(arc_other)) => {
                arc_self.relative_eq(arc_other, epsilon, max_relative)
            }
            _ => false,
        }
    }
}

impl<Frame> PathSegment<Frame> {
    pub fn length(&self) -> f32 {
        match self {
            PathSegment::LineSegment(line_segment) => line_segment.length(),
            PathSegment::Arc(arc) => arc.length(),
        }
    }
}

#[derive(
    Clone, Debug, Default, Serialize, PathSerialize, PathDeserialize, PathIntrospect, Deserialize,
)]
pub struct PlannedPath<Frame> {
    pub path: Option<Vec<PathSegment<Frame>>>,
}
