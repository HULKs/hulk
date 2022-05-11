use macros::SerializeHierarchy;
use serde::{Deserialize, Serialize};

use super::{BodyJoints, HeadJoints};

#[derive(Debug, Clone, Default, Copy, Deserialize, Serialize, SerializeHierarchy)]
pub struct DispatchingHeadPositions {
    pub positions: HeadJoints,
}

#[derive(Debug, Clone, Default, Copy, Deserialize, Serialize, SerializeHierarchy)]
pub struct DispatchingBodyPositions {
    pub positions: BodyJoints,
}
