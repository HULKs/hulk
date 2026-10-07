use std::time::Duration;

use ros_z::{Message, time::Time};
use serde::{Deserialize, Serialize};

pub const MAXIMUM_FALL_DETECTION_AGE: Duration = Duration::from_millis(100);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Message)]
pub enum Posture {
    Upright,
    Falling,
    Fallen { ready_for_standup: bool },
    StandingUp,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Message)]
pub struct FallDetection {
    /// Source time of the sensor sample this posture was derived from.
    pub time: Time,
    pub posture: Posture,
}

impl FallDetection {
    pub fn is_fresh(&self, now: Time) -> bool {
        self.time <= now && now.duration_since(self.time) <= MAXIMUM_FALL_DETECTION_AGE
    }

    pub fn is_upright(&self, now: Time) -> bool {
        self.is_fresh(now) && self.posture == Posture::Upright
    }
}
