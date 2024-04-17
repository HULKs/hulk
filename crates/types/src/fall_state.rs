use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize, SerializeHierarchy)]
pub enum Side {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize, SerializeHierarchy)]
pub enum Direction {
    Forward { side: Side },
    Backward { side: Side },
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize, SerializeHierarchy)]
pub enum Kind {
    FacingDown,
    FacingUp,
    Sitting,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize, SerializeHierarchy)]
pub enum FallState {
    #[default]
    Upright,
    Falling {
        start_time: SystemTime,
        direction: Direction,
    },
    Fallen {
        kind: Kind,
    },
    StandingUp {
        start_time: SystemTime,
        kind: Kind,
    },
}
