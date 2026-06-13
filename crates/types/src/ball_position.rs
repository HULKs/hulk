use std::{ops::Mul, time::Duration};

use ros_z::time::Time;
use serde::{Deserialize, Serialize};

use coordinate_systems::World;
use linear_algebra::{Isometry2, Point2, Vector2};
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};

#[derive(
    Debug,
    Clone,
    Copy,
    PathDeserialize,
    PathSerialize,
    PathIntrospect,
    Serialize,
    Deserialize,
    ros_z::Message,
)]
pub struct BallPosition<Frame> {
    pub position: Point2<Frame>,
    pub velocity: Vector2<Frame>,
    #[path_serde(skip)]
    pub last_seen: Time,
}

impl<Frame> BallPosition<Frame> {
    pub fn age_at(&self, now: Time) -> Option<Duration> {
        if now < self.last_seen {
            return None;
        }

        Some(now.duration_since(self.last_seen))
    }

    pub fn from_network_ball(
        network_ball: hsl_network_messages::BallPosition<Frame>,
        message_time: Time,
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
    pub position: Point2<World>,
    pub velocity: Vector2<World>,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Deserialize,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
    ros_z::Message,
)]
pub struct HypotheticalBallPosition<Frame> {
    pub position: Point2<Frame>,
    pub validity: f32,
}
