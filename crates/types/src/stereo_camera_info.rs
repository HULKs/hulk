use ros2::sensor_msgs::camera_info::CameraInfo;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, ros_z::Message)]
pub struct StereoCameraInfo {
    pub left: CameraInfo,
    pub right: CameraInfo,
}
