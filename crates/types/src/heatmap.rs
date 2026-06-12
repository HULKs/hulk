use ros_z::Message;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize, Message)]
pub struct Heatmap {
    pub length: u32,
    pub width: u32,
    pub values: Vec<f32>,
}
