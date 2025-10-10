use ros2::{
    sensor_msgs::{camera_info::CameraInfo, image::Image, imu::Imu, magnetic_field::MagneticField},
    std_msgs::header::Header,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct RGBDSensors {
    header: Header,

    #[serde(rename = "rgbCameraInfo")]
    rgb_camera_info: CameraInfo,
    #[serde(rename = "depthCameraInfo")]
    depth_camera_info: CameraInfo,

    rgb: Image,
    depth: Image,

    imu: Imu,

    mag: MagneticField,
}
