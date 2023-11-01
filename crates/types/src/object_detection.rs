use geometry::rectangle::Rectangle;
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, SerializeHierarchy)]
pub enum DetectedObject {
    Ball,
    Robot,
    GoalPost,
    PenaltySpot,
}

impl DetectedObject {
    pub fn from_u8(index: u8) -> Option<DetectedObject> {
        // 0 is background
        match index {
            1 => Some(DetectedObject::Ball),
            2 => Some(DetectedObject::Robot),
            3 => Some(DetectedObject::GoalPost),
            4 => Some(DetectedObject::PenaltySpot),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, SerializeHierarchy)]
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

    pub fn iou(&self, other: &Self) -> f32 {
        let intersection = self.bounding_box.rectangle_intersection(other.bounding_box);
        let union = self.bounding_box.area() + other.bounding_box.area();

        intersection / (union - intersection)
    }
}
