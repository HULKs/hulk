use ros_z::Message;
use ros2::sensor_msgs::image::Image;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, Message)]
pub struct StereoImagePair {
    pub frame_identifier: u32,
    pub left: Image,
    pub right: Image,
}
