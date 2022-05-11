use macros::SerializeHierarchy;
use serde::{Deserialize, Serialize};

use super::BodyJoints;

#[derive(Clone, Debug, Default, Serialize, SerializeHierarchy, Deserialize)]
pub struct StandUpFrontPositions {
    pub positions: BodyJoints,
}
