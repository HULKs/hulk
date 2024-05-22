use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use coordinate_systems::Field;
use linear_algebra::{Point2, Vector2};
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};

#[derive(
    Debug, Clone, Copy, PathDeserialize, PathSerialize, PathIntrospect, Serialize, Deserialize,
)]
pub struct BallPosition<Frame> {
    pub position: Point2<Frame>,
    pub velocity: Vector2<Frame>,
    pub last_seen: SystemTime,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Deserialize,
    PathDeserialize,
    PathIntrospect,
    PathSerialize,
    Serialize,
)]
pub struct SimulatorBallState {
    pub position: Point2<Field>,
    pub velocity: Vector2<Field>,
}

#[derive(
    Clone, Copy, Debug, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct HypotheticalBallPosition<Frame> {
    pub position: Point2<Frame>,
    pub validity: f32,
}
