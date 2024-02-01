use geometry::rectangle::Rectangle;
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

#[derive(Debug, Clone, Serialize, Deserialize, SerializeHierarchy)]
pub struct BoundingBox {
    pub bounding_box: Rectangle,
    pub score: f32,
}

impl BoundingBox {
    pub fn new(score: f32, bounding_box: Rectangle) -> Self {
        Self {
            bounding_box,
            score,
        }
    }

    pub fn iou(&self, other: &Self) -> f32 {
        let intersection = self.bounding_box.rectangle_intersection(other.bounding_box);
        let union = self.bounding_box.area() + other.bounding_box.area();

        intersection / (union - intersection)
    }
}
