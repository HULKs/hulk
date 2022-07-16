use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

use crate::Side;

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct InitialPose {
    pub center_line_offset_x: f32,
    pub side: Side,
}
