use ros_z::Message;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default, Message)]
#[message(name = "custom_msgs::RobotStatus")]
pub struct RobotStatus {
    pub robot_id: String,
    pub battery_percentage: f64,
    pub position_x: f64,
    pub position_y: f64,
    pub is_moving: bool,
}

impl ros_z::msg::WireMessage for RobotStatus {
    type Codec = ros_z::msg::SerdeCdrCodec<RobotStatus>;
}
