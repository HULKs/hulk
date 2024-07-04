use std::time::SystemTime;

use linear_algebra::{Point2, Vector2};
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, Copy, PathDeserialize, PathSerialize, PathIntrospect, Serialize, Deserialize,
)]
pub struct BallPosition<Frame> {
    pub position: Point2<Frame>,
    pub velocity: Vector2<Frame>,
    pub last_seen: SystemTime,
}
