use super::HeadJoints;
use macros::SerializeHierarchy;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize, SerializeHierarchy)]
pub struct FallProtection {
    pub head_position: HeadJoints,
    pub head_stiffness: f32,
}
