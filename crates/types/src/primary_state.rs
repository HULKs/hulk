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
    Animation,
    AnimationStiff,
    Unstiff,
    Initial,
    Ready,
    Set,
    Playing,
    Penalized,
    Finished,
    Calibration,
    Standby,
}
