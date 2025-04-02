use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

#[derive(
    Clone,
    Copy,
    Default,
    Debug,
    Deserialize,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
    PartialEq,
)]
pub enum PenaltyShotDirection {
    #[default]
    NotMoving,
    Center,
    Left,
    Right,
}
