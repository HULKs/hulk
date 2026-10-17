use ros_z::{Message, time::Time};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Message)]
pub enum Posture {
    Upright,
    Falling,
    Fallen { is_ready_for_standup: bool },
    StandingUp,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Message)]
pub struct FallDetection {
    /// Source time of the sensor sample this posture was derived from.
    pub time: Time,
    /// Consumers must not use this posture after this time.
    pub valid_until: Time,
    pub posture: Posture,
}

impl FallDetection {
    pub fn is_valid(&self, now: Time) -> bool {
        self.time <= now && now <= self.valid_until
    }
}
