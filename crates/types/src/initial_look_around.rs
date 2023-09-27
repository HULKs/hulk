use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

use crate::support_foot::Side;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, SerializeHierarchy)]
pub enum Mode {
    Center { moving_towards: Side },
    Left,
    Right,
    HalfwayLeft { moving_towards: Side },
    HalfwayRight { moving_towards: Side },
}

impl Default for Mode {
    fn default() -> Self {
        Self::Center {
            moving_towards: Side::Left,
        }
    }
}
