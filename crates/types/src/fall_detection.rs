use std::time::Duration;

use ros_z::{Message, time::Time};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, Message)]
pub enum Posture {
    #[default]
    Unknown,
    Upright,
    Falling,
    Fallen,
}

/// Physical estimate. Recovery authorization belongs to Motion, not this message.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Message)]
pub struct FallDetection {
    pub time: Time,
    pub sample_time: Time,
    pub posture: Posture,
    pub tilt: f32,
    pub angular_speed: f32,
    pub ready_for_walk: bool,
}

impl FallDetection {
    pub fn is_fresh(&self, now: Time, maximum_age: Duration) -> bool {
        self.time <= now
            && self.sample_time <= now
            && now.duration_since(self.time) <= maximum_age
            && now.duration_since(self.sample_time) <= maximum_age
            && self.posture != Posture::Unknown
    }
}
