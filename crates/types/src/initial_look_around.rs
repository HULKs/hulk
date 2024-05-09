use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

use crate::support_foot::Side;

#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub enum BallSearchLookAround {
    Center { moving_towards: Side },
    Left,
    Right,
    HalfwayLeft { moving_towards: Side },
    HalfwayRight { moving_towards: Side },
}

impl Default for BallSearchLookAround {
    fn default() -> Self {
        Self::Center {
            moving_towards: Side::Left,
        }
    }
}

#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
pub struct QuickLookAround {
    pub mode: BallSearchLookAround,
}

#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub enum InitialLookAround {
    Left,
    Right,
}

impl Default for InitialLookAround {
    fn default() -> Self {
        Self::Left
    }
}

#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub enum LookAroundMode {
    Center,
    BallSearch(BallSearchLookAround),
    QuickSearch(QuickLookAround),
    Initial(InitialLookAround),
}
