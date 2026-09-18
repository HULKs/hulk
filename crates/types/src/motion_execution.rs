use ros_z::{Message, time::Time};
use serde::{Deserialize, Serialize};

pub const MOTION_EXECUTION_TOPIC: &str = "motion/execution";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Message)]
pub enum MotionPhase {
    Damping,
    Preparing,
    Normal,
    Recovering { fast: bool },
    Settling,
    Fault,
}

#[derive(Clone, Debug, Serialize, Deserialize, Message)]
pub struct MotionExecution {
    pub time: Time,
    pub generation: u64,
    pub phase: MotionPhase,
    pub recovery_started_at: Option<Time>,
    pub recovery_progress: Option<f32>,
    pub fault: Option<String>,
}

impl MotionExecution {
    pub fn is_fresh(&self, now: Time) -> bool {
        self.time <= now && now.duration_since(self.time) <= std::time::Duration::from_millis(100)
    }
}
