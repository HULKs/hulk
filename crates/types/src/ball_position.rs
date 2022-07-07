use std::time::{SystemTime, UNIX_EPOCH};

use nalgebra::Point2;
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

#[derive(Clone, Copy, Serialize, Deserialize, SerializeHierarchy, Debug)]
pub struct BallPosition {
    pub position: Point2<f32>,
    #[leaf]
    pub last_seen: SystemTime,
}

impl Default for BallPosition {
    fn default() -> Self {
        Self {
            position: Default::default(),
            last_seen: UNIX_EPOCH,
        }
    }
}
