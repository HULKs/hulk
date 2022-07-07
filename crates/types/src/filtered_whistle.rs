use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct FilteredWhistle {
    pub is_detected: bool,
    pub started_this_cycle: bool,
    #[leaf]
    pub last_detection: Option<SystemTime>,
}
