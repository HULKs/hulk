use macros::SerializeHierarchy;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Deserialize)]
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

#[derive(Clone, Debug, Default, Serialize, SerializeHierarchy, Deserialize)]
pub struct SupportFoot {
    #[leaf]
    pub support_side: Side,
    pub changed_this_cycle: bool,
}
