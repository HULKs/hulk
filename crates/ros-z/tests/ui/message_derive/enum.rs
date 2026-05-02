use ros_z::Message;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
enum RobotState {
    Idle,
    Error(String),
}

fn main() {}
