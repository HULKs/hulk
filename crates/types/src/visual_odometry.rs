use ros_z::Message;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Message)]
pub struct TriangulatedFeature {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}
