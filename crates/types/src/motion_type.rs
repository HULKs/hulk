use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use ros_z::Message;
use serde::{Deserialize, Serialize};

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Deserialize,
    Eq,
    PartialEq,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
    Message,
)]
pub enum MotionType {
    #[default]
    Damping,
    Prepare,
    Stand,
    StandUp,
    Kick,
    Walk,
}
