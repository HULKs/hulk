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
    ros_z::Message,
)]
pub enum PrimaryState {
    #[default]
    Safe,
    Prepare,
    Stop,
    Initial,
    Ready,
    Set,
    Playing,
    Penalized,
    Finished,
}
