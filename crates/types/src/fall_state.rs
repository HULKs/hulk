use std::time::SystemTime;

use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

#[derive(
    Clone,
    Copy,
    Debug,
    Deserialize,
    PartialEq,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
pub enum Side {
    Left,
    Right,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Deserialize,
    PartialEq,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
pub enum Direction {
    Forward { side: Side },
    Backward { side: Side },
}

#[derive(
    Clone,
    Copy,
    Debug,
    Deserialize,
    PartialEq,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
pub enum Kind {
    FacingDown,
    FacingUp,
    Sitting,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Deserialize,
    PartialEq,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
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
