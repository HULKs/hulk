use coordinate_systems::Pixel;
use geometry::rectangle::Rectangle;
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, SerializeHierarchy)]
pub struct BoundingBox {
    pub area: Rectangle<Pixel>,
    pub score: f32,
}

impl BoundingBox {
    pub fn intersection_over_union(&self, other: &Self) -> f32 {
        let intersection = self.area.rectangle_intersection(other.area);
        let union = self.area.area() + other.area.area();

        intersection / (union - intersection)
    }
}
