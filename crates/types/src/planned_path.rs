use approx::{AbsDiffEq, RelativeEq};
use coordinate_systems::Point2;
use geometry::{arc::Arc, direction::Direction, line_segment::LineSegment};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

use crate::coordinate_systems::Ground;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, SerializeHierarchy)]
pub enum PathSegment {
    LineSegment(LineSegment<Ground>),
    Arc(Arc<Ground>, Direction),
}

pub fn direct_path(start: Point2<Ground>, destination: Point2<Ground>) -> Vec<PathSegment> {
    vec![PathSegment::LineSegment(LineSegment(start, destination))]
}

impl AbsDiffEq for PathSegment {
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
            (
                PathSegment::Arc(arc_self, direction_self),
                PathSegment::Arc(arc_other, direction_other),
            ) => direction_self == direction_other && arc_self.abs_diff_eq(arc_other, epsilon),
            _ => false,
        }
    }
}

impl RelativeEq for PathSegment {
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
            (
                PathSegment::Arc(arc_self, direction_self),
                PathSegment::Arc(arc_other, direction_other),
            ) => {
                direction_self == direction_other
                    && arc_self.relative_eq(arc_other, epsilon, max_relative)
            }
            _ => false,
        }
    }
}

impl PathSegment {
    pub fn length(&self) -> f32 {
        match self {
            PathSegment::LineSegment(line_segment) => line_segment.norm(),
            PathSegment::Arc(arc, direction) => arc.length(*direction),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, SerializeHierarchy, Deserialize)]
pub struct PlannedPath {
    pub path: Option<Vec<PathSegment>>,
}
