use std::{ops::Mul, time::SystemTime};

use serde::{Deserialize, Serialize};

use coordinate_systems::Field;
use linear_algebra::{Isometry2, Point2, Vector2};
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};

#[derive(
    Debug, Clone, Copy, PathDeserialize, PathSerialize, PathIntrospect, Serialize, Deserialize,
)]
pub struct BallPosition<Frame> {
    pub position: Point2<Frame>,
    pub velocity: Vector2<Frame>,
    pub last_seen: SystemTime,
}

impl<Frame> BallPosition<Frame> {
    pub fn from_network_ball(
        network_ball: hsl_network_messages::BallPosition<Frame>,
        message_time: SystemTime,
    ) -> Self {
        Self {
            position: network_ball.position,
            velocity: Vector2::zeros(),
            last_seen: message_time - network_ball.age,
        }
    }
}

impl<From, To> Mul<BallPosition<From>> for Isometry2<From, To> {
    type Output = BallPosition<To>;

    fn mul(self, rhs: BallPosition<From>) -> Self::Output {
        BallPosition {
            position: self * rhs.position,
            velocity: self * rhs.velocity,
            last_seen: rhs.last_seen,
        }
    }
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
