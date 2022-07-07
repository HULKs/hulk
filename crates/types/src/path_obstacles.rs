use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

use crate::{Arc, Circle, LineSegment, Orientation};

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
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
        if let PathObstacleShape::Circle(circle) = self {
            Some(circle)
        } else {
            None
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, SerializeHierarchy)]
pub struct PathObstacle {
    #[leaf]
    pub shape: PathObstacleShape,
    pub nodes: Vec<usize>,
    #[leaf]
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
