use coordinate_systems::Ground;
use linear_algebra::Pose2;
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

use crate::{motion_command::KickVariant, support_foot::Side};

#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct KickDecision {
    pub variant: KickVariant,
    pub kicking_side: Side,
    pub kick_pose: Pose2<Ground>,
    pub strength: f32,
}
