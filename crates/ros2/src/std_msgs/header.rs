use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use ros_z::Message;
use serde::{Deserialize, Serialize};

use crate::builtin_interfaces::time::Time;

/// Standard metadata for higher-level stamped data types.
/// This is generally used to communicate timestamped data
/// in a particular coordinate frame.
#[repr(C)]
#[derive(
    Clone,
    Debug,
    Default,
    Serialize,
    Deserialize,
    PathIntrospect,
    PathSerialize,
    PathDeserialize,
    Message,
)]
pub struct Header {
    /// Two-integer timestamp that is expressed as seconds and nanoseconds.
    pub stamp: Time,

    /// Transform frame with which this data is associated.
    pub frame_id: String,
}
