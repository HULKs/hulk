use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

use crate::support_foot::Side;

#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub enum Mode {
    Center { moving_towards: Side },
    Left,
    Right,
    HalfwayLeft { moving_towards: Side },
    HalfwayRight { moving_towards: Side },
    InitialLeft,
    InitialRight,
}

impl Default for Mode {
    fn default() -> Self {
        Self::Center {
            moving_towards: Side::Left,
        }
    }
}
