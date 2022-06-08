use macros::SerializeHierarchy;
use serde::{Deserialize, Serialize};

use super::{BodyJoints, HeadJoints};

#[derive(Clone, Debug, Default, Serialize, SerializeHierarchy, Deserialize)]
pub struct StandUpFrontPositions {
    pub body_positions: BodyJoints,
    pub head_positions: HeadJoints,
}
