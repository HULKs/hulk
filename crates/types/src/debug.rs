use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

#[derive(
    Clone,
    Copy,
    Serialize,
    Debug,
    Default,
    Deserialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
pub enum LedDebug {
    #[default]
    Temperature,
    Walking,
    Battery,
}
