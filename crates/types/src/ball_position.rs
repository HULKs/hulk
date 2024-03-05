use std::time::{SystemTime, UNIX_EPOCH};

use nalgebra::{Point2, Vector2};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

#[derive(Clone, Copy, Serialize, Deserialize, SerializeHierarchy, Debug)]
pub struct BallPosition {
    pub position: Point2<f32>,
    pub rest_position: Point2<f32>,
    pub velocity: Vector2<f32>,
    pub last_seen: SystemTime,
    pub is_resting: bool,
}

impl Default for BallPosition {
    fn default() -> Self {
        Self {
            position: Default::default(),
            rest_position: Default::default(),
            velocity: Default::default(),
            last_seen: UNIX_EPOCH,
            is_resting: false,
        }
    }
}
