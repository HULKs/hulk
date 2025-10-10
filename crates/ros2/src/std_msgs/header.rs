/// Standard metadata for higher-level stamped data types.
/// This is generally used to communicate timestamped data
/// in a particular coordinate frame.

/// Two-integer timestamp that is expressed as seconds and nanoseconds.

use serde::{Deserialize, Serialize};

use crate::builtin_interfaces::time::Time;

#[derive(Debug, Serialize, Deserialize)]
pub struct Header {
    pub stamp: Time,

    /// Transform frame with which this data is associated.
    pub frame_id: String,
}
