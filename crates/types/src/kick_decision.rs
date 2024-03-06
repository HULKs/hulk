use coordinate_systems::Ground;
use linear_algebra::Pose;
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

use crate::{motion_command::KickVariant, support_foot::Side};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, SerializeHierarchy)]
pub struct KickDecision {
    pub variant: KickVariant,
    pub kicking_side: Side,
    pub kick_pose: Pose<Ground>,
    pub strength: f32,
}
