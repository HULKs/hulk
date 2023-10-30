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

impl DetectedObject {
    pub fn from_u8(index: u8) -> Option<DetectedObject> {
        match index {
            1 => Some(DetectedObject::Robot),
            2 => Some(DetectedObject::Ball),
            3 => Some(DetectedObject::GoalPost),
            4 => Some(DetectedObject::PenaltySpot),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, SerializeHierarchy)]
pub struct BoundingBox {
    pub bounding_box: Rectangle,
    pub class: DetectedObject,
    pub score: f32,
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
