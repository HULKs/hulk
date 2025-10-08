use ros2::{
    sensor_msgs::{camera_info::CameraInfo, image::Image, imu::Imu, magnetic_field::MagneticField},
    std_msgs::header::Header,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct RGBDSensors {
    pub header: Header,

    #[serde(rename = "rgbCameraInfo")]
    pub rgb_camera_info: CameraInfo,
    #[serde(rename = "depthCameraInfo")]
    pub depth_camera_info: CameraInfo,

    pub rgb: Image,
    pub depth: Image,

    pub imu: Imu,

    pub mag: MagneticField,
}
