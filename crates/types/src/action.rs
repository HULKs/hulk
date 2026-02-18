use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

use crate::field_dimensions::Side;

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
