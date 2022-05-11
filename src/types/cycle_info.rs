use std::time::{Duration, SystemTime, UNIX_EPOCH};

use macros::SerializeHierarchy;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, SerializeHierarchy)]
pub struct CycleInfo {
    pub start_time: SystemTime,
    pub last_cycle_duration: Duration,
}

impl Default for CycleInfo {
    fn default() -> Self {
        Self {
            start_time: UNIX_EPOCH,
            last_cycle_duration: Default::default(),
        }
    }
}
