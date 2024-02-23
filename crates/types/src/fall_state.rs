use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, SerializeHierarchy)]
pub enum Side {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, SerializeHierarchy)]
pub enum FallDirection {
    Forward { side: Side },
    Backward { side: Side },
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, SerializeHierarchy)]
pub enum Facing {
    Down,
    Up,
    Sitting,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, SerializeHierarchy, Default)]
pub enum FallState {
    #[default]
    Upright,
    Falling {
        start_time: SystemTime,
        direction: FallDirection,
    },
    Fallen {
        facing: Facing,
    },
    Sitting {
        start_time: SystemTime,
    },
}
