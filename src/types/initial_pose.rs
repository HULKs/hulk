use crate::types::Side;
use macros::SerializeHierarchy;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct InitialPose {
    pub center_line_offset_x: f32,
    #[leaf]
    pub side: Side,
}
