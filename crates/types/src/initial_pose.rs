use serde::{Deserialize, Serialize};

use crate::support_foot::Side;

#[derive(Clone, Debug, Default, Deserialize, Serialize, ros_z::Message)]
pub struct InitialPose {
    pub center_line_offset_x: f32,
    pub side: Side,
}
