use coordinate_systems::Framed;
use nalgebra::Point2;
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

use crate::coordinate_systems::{Ground, Pixel};

#[derive(Default, Clone, Debug, Deserialize, Serialize, SerializeHierarchy)]
pub struct DetectedFeet {
    pub positions: Vec<Framed<Ground, Point2<f32>>>,
}

#[derive(Default, Clone, Debug, Deserialize, Serialize, SerializeHierarchy)]
pub struct ClusterPoint {
    pub pixel_coordinates: Framed<Pixel, Point2<u16>>,
    pub position_in_ground: Framed<Ground, Point2<f32>>,
}

#[derive(Default, Clone, Debug, Deserialize, Serialize, SerializeHierarchy)]
pub struct CountedCluster {
    pub mean: Framed<Ground, Point2<f32>>,
    pub samples: usize,
}
