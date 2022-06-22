use macros::SerializeHierarchy;
use nalgebra::Point2;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize, SerializeHierarchy)]
pub struct Limb {
    pub pixel_polygon: Vec<Point2<f32>>,
}
