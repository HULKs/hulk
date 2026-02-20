use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

#[derive(
    Debug,
    Clone,
    Copy,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
)]
pub enum Action {
    WalkToBall,
}
