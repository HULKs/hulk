use serde::{Deserialize, Serialize};

use crate::support_side::Side;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, ros_z::Message)]
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

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, PartialEq, ros_z::Message)]
pub struct QuickLookAround {
    pub mode: BallSearchLookAround,
}

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, PartialEq, ros_z::Message)]
pub enum InitialLookAround {
    #[default]
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, ros_z::Message)]
pub enum LookAroundMode {
    Center,
    BallSearch(BallSearchLookAround),
    QuickSearch(QuickLookAround),
    Initial(InitialLookAround),
}
