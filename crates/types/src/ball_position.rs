use std::time::{SystemTime, UNIX_EPOCH};

use nalgebra::{Point2, Vector2};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, SerializeHierarchy)]
pub struct BallPosition {
    pub position: Point2<f32>,
    pub velocity: Vector2<f32>,
    pub last_seen: SystemTime,
}

impl Default for BallPosition {
    fn default() -> Self {
        Self {
            position: Default::default(),
            velocity: Default::default(),
            last_seen: UNIX_EPOCH,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, SerializeHierarchy)]
pub struct HypotheticalBallPosition {
    pub position: Point2<f32>,
    pub validity: f32,
}
