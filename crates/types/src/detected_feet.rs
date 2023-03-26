use nalgebra::Point2;
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

#[derive(Default, Clone, Debug, Deserialize, Serialize, SerializeHierarchy)]
pub struct DetectedFeet {
    pub positions: Vec<Point2<f32>>,
}

#[derive(Default, Clone, Debug, Deserialize, Serialize, SerializeHierarchy)]
pub struct ClusterPoint {
    pub pixel_coordinates: Point2<u16>,
    pub position_in_ground: Point2<f32>,
}
