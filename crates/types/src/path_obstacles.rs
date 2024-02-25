use std::collections::HashSet;

use geometry::{arc::Arc, circle::Circle, line_segment::LineSegment, orientation::Direction};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

use crate::coordinate_systems::Ground;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, SerializeHierarchy)]
pub enum PathObstacleShape {
    Circle(Circle<Ground>),
    LineSegment(LineSegment<Ground>),
}

impl PathObstacleShape {
    pub fn intersects_line_segment(&self, line_segment: LineSegment<Ground>) -> bool {
        match self {
            PathObstacleShape::Circle(circle) => circle.intersects_line_segment(&line_segment),
            PathObstacleShape::LineSegment(obstacle_line_segment) => {
                obstacle_line_segment.intersects_line_segment(line_segment)
            }
        }
    }

    pub fn overlaps_arc(&self, arc: Arc<Ground>, orientation: Direction) -> bool {
        match self {
            PathObstacleShape::Circle(circle) => circle.overlaps_arc(arc, orientation),
            PathObstacleShape::LineSegment(line_segment) => {
                line_segment.overlaps_arc(arc, orientation)
            }
        }
    }

    pub fn as_circle(&self) -> Option<&Circle<Ground>> {
        match self {
            PathObstacleShape::Circle(circle) => Some(circle),
            _ => None,
        }
    }

    pub fn as_circle_mut(&mut self) -> Option<&mut Circle<Ground>> {
        match self {
            PathObstacleShape::Circle(circle) => Some(circle),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, SerializeHierarchy)]
pub struct PathObstacle {
    pub shape: PathObstacleShape,
    pub nodes: Vec<usize>,
    pub populated_connections: HashSet<usize>,
}

impl From<PathObstacleShape> for PathObstacle {
    fn from(shape: PathObstacleShape) -> Self {
        Self {
            shape,
            nodes: vec![],
            populated_connections: HashSet::new(),
        }
    }
}

impl From<Circle<Ground>> for PathObstacle {
    fn from(shape: Circle<Ground>) -> Self {
        Self::from(PathObstacleShape::Circle(shape))
    }
}
impl From<LineSegment<Ground>> for PathObstacle {
    fn from(shape: LineSegment<Ground>) -> Self {
        Self::from(PathObstacleShape::LineSegment(shape))
    }
}
