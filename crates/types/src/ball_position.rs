use std::time::{SystemTime, UNIX_EPOCH};

use coordinate_systems::Framed;
use nalgebra::{Point2, Vector2};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

#[derive(Clone, Copy, Serialize, Deserialize, SerializeHierarchy, Debug)]
pub struct BallPosition<Frame> {
    pub position: Framed<Frame, Point2<f32>>,
    pub velocity: Framed<Frame, Vector2<f32>>,
    pub last_seen: SystemTime,
}

impl<Frame> Default for BallPosition<Frame> {
    fn default() -> Self {
        Self {
            position: Default::default(),
            velocity: Default::default(),
            last_seen: UNIX_EPOCH,
        }
    }
}
