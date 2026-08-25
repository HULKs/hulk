use std::time::SystemTime;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub enum Side {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub enum FallingDirection {
    Forward { side: Side },
    Backward { side: Side },
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub enum FallenKind {
    FacingDown,
    FacingUp,
    Sitting,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum StandUpSpeed {
    Default,
    Slow,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub enum FallState {
    Upright,
    Falling {
        start_time: SystemTime,
        direction: FallingDirection,
    },
    Fallen {
        kind: FallenKind,
    },
    StandingUp {
        start_time: SystemTime,
        kind: FallenKind,
    },
}
