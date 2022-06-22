use macros::SerializeHierarchy;
use serde::{Deserialize, Serialize};

use super::Joints;

#[derive(Clone, Debug, Default, Serialize, SerializeHierarchy, Deserialize)]
pub struct SitDownJoints {
    pub positions: Joints,
    pub stiffnesses: Joints,
}
