use std::time::SystemTime;

use macros::SerializeHierarchy;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct FilteredWhistle {
    pub is_detected: bool,
    pub started_this_cycle: bool,
    #[leaf]
    pub last_detection: Option<SystemTime>,
}
