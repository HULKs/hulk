use ros_z::{Message, time::Time};
use serde::{Deserialize, Serialize};

pub const HARDWARE_STATUS_TOPIC: &str = "hardware_interface/status";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Message)]
pub enum ControlMode {
    Damping,
    Prepare,
    Custom,
}

#[derive(Clone, Debug, Serialize, Deserialize, Message)]
pub struct HardwareStatus {
    pub time: Time,
    pub desired: ControlMode,
    pub acknowledged: Option<ControlMode>,
    pub command_time: Option<Time>,
    pub fault: Option<String>,
}
