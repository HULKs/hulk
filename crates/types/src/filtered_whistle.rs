use std::time::SystemTime;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, ros_z::Message)]
pub struct FilteredWhistle {
    pub is_detected: bool,
    pub last_detection: Option<SystemTime>,
}
