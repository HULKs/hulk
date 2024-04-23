use coordinate_systems::Pixel;
use geometry::rectangle::Rectangle;
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct BoundingBox {
    pub area: Rectangle<Pixel>,
    pub confidence: f32,
}

impl BoundingBox {
    pub fn intersection_over_union(&self, other: &Self) -> f32 {
        let intersection = self.area.rectangle_intersection(other.area);
        let union = self.area.area() + other.area.area();

        intersection / (union - intersection)
    }
}
