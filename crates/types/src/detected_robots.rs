use nalgebra::{Point2, Vector2};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

#[derive(Default, Clone, Debug, Deserialize, Serialize, SerializeHierarchy)]
pub struct DetectedRobots {
    pub in_image: Vec<BoundingBox>,
    pub on_ground: Vec<Point2<f32>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, SerializeHierarchy)]
pub struct BoundingBox {
    pub center: Point2<f32>,
    pub size: Vector2<f32>,
    pub probability: f32,
    pub distance: f32,
}
