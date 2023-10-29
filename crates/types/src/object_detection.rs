use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

use crate::geometry::Rectangle;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, SerializeHierarchy)]
pub enum DetectedObject {
    Robot,
    Ball,
    GoalPost,
    PenaltySpot,
}

#[derive(Debug, Clone, Serialize, Deserialize, SerializeHierarchy)]
pub struct BoundingBox {
    bounding_box: Rectangle,
    class: DetectedObject,
    score: f32,
}

impl BoundingBox {
    pub fn new(class: DetectedObject, score: f32, bounding_box: Rectangle) -> Self {
        Self {
            bounding_box,
            class,
            score,
        }
    }
}
