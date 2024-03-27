use std::time::Duration;

use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

use crate::{joints::Joints, motor_commands::MotorCommands};

#[derive(Default, Clone, Copy, Debug, Deserialize, Serialize, SerializeHierarchy)]
pub struct MotionFileState {
    pub commands: MotorCommands<Joints<f32>>,
    pub remaining_duration: Duration,
}
