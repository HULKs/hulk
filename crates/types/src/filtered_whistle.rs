use std::time::SystemTime;

use path_serde::{PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize, PathSerialize, PathIntrospect)]
pub struct FilteredWhistle {
    pub is_detected: bool,
    pub started_this_cycle: bool,
    pub last_detection: Option<SystemTime>,
}
