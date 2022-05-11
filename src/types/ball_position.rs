use std::time::{SystemTime, UNIX_EPOCH};

use macros::SerializeHierarchy;
use nalgebra::Point2;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Serialize, Deserialize, SerializeHierarchy, Debug)]
pub struct BallPosition {
    pub position: Option<Point2<f32>>,
    pub last_seen: SystemTime,
}

impl Default for BallPosition {
    fn default() -> Self {
        Self {
            position: None,
            last_seen: UNIX_EPOCH,
        }
    }
}
