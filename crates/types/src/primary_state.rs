use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Deserialize,
    Eq,
    Hash,
    PartialEq,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
pub enum PrimaryState {
    #[default]
    Unstiff,
    Animation {
        stiff: bool,
    },
    Initial,
    Ready,
    Set,
    Playing,
    Penalized,
    Finished,
    Calibration,
    Standby,
}
