use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

use crate::{Arc, Circle, LineSegment, Orientation};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, SerializeHierarchy)]
pub enum PathObstacleShape {
    Circle(Circle),
    LineSegment(LineSegment),
}

impl PathObstacleShape {
    pub fn intersects_line_segment(&self, line_segment: LineSegment) -> bool {
        match self {
            PathObstacleShape::Circle(circle) => circle.intersects_line_segment(&line_segment),
            PathObstacleShape::LineSegment(obstacle_line_segment) => {
                obstacle_line_segment.intersects_line_segment(line_segment)
            }
        }
    }

    pub fn overlaps_arc(&self, arc: Arc, orientation: Orientation) -> bool {
        match self {
            PathObstacleShape::Circle(circle) => circle.overlaps_arc(arc, orientation),
            PathObstacleShape::LineSegment(line_segment) => {
                line_segment.overlaps_arc(arc, orientation)
            }
        }
    }

    pub fn as_circle(&self) -> Option<&Circle> {
        match self {
            PathObstacleShape::Circle(circle) => Some(circle),
            _ => None,
        }
    }

    pub fn as_circle_mut(&mut self) -> Option<&mut Circle> {
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

impl From<Circle> for PathObstacle {
    fn from(shape: Circle) -> Self {
        Self::from(PathObstacleShape::Circle(shape))
    }
}
impl From<LineSegment> for PathObstacle {
    fn from(shape: LineSegment) -> Self {
        Self::from(PathObstacleShape::LineSegment(shape))
    }
}
