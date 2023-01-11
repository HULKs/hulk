use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, SerializeHierarchy)]
pub enum Side {
    Left,
    Right,
}

impl Side {
    pub fn opposite(&self) -> Side {
        match self {
            Side::Left => Self::Right,
            Side::Right => Self::Left,
        }
    }
}

impl Default for Side {
    fn default() -> Self {
        Self::Left
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct SupportFoot {
    pub support_side: Option<Side>,
    pub changed_this_cycle: bool,
}
