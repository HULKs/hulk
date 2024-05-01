use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

#[derive(
    Clone, Copy, Debug, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub enum PenaltyShotDirection {
    NotMoving,
    Left,
    Right,
}
