#[cfg(feature = "pyo3")]
use std::time::Duration;

#[cfg(feature = "pyo3")]
use ros_z_msgs::{builtin_interfaces::Time, sensor_msgs::RegionOfInterest};
use ros_z_msgs::{
    sensor_msgs::{CameraInfo, Image, Imu, MagneticField},
    std_msgs::Header,
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "pyo3")]
use pyo3::{pyclass, pymethods, pymodule};

#[repr(C)]
#[cfg_attr(feature = "pyo3", pyclass(frozen))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RGBDSensors {
    pub header: Header,

    #[serde(rename = "rgbCameraInfo")]
    pub rgb_camera_info: CameraInfo,
    #[serde(rename = "depthCameraInfo")]
    pub depth_camera_info: CameraInfo,

    pub rgb: Box<Image>,
    pub depth: Box<Image>,

    pub imu: Imu,

    pub mag: MagneticField,
}

#[cfg(feature = "pyo3")]
#[pymethods]
impl RGBDSensors {
    #[new]
    pub fn from_mujoco(time: f32, rgb: Vec<u8>, depth: Vec<u16>, height: u32, width: u32) -> Self {
        let simulation_duration = Duration::from_secs_f32(time);

        let header = Header {
            stamp: Time {
                sec: simulation_duration.as_secs() as i32,
                nanosec: simulation_duration.subsec_nanos(),
            },
            frame_id: "".to_string(),
        };

        let rgb_camera_info = CameraInfo {
            header: header.clone(),
            width,
            height,
            distortion_model: "".to_string(),
            roi: RegionOfInterest {
                x_offset: 0,
                y_offset: 0,
                height: 0,
                width: 0,
                do_rectify: false,
            },
            d: Default::default(),
            k: Default::default(),
            r: Default::default(),
            p: Default::default(),
            binning_x: Default::default(),
            binning_y: Default::default(),
        };

        let depth_camera_info = rgb_camera_info.clone();

        let rgb_image = Image {
            header: header.clone(),
            height,
            width,
            encoding: "rgb8".to_string(),
            is_bigendian: 0,
            step: width * 3,
            data: rgb.into(),
        };

        let depth_image = Image {
            header: header.clone(),
            height,
            width,
            encoding: "mono16".to_string(),
            is_bigendian: 1,
            step: width * 2,
            data: depth
                .iter()
                .flat_map(|x| x.to_be_bytes())
                .collect::<Vec<_>>()
                .into(),
        };

        Self {
            header: header.clone(),
            rgb_camera_info,
            depth_camera_info,
            rgb: Box::new(rgb_image),
            depth: Box::new(depth_image),
            imu: Imu {
                header: header.clone(),
                ..Default::default()
            },
            mag: MagneticField {
                header: header.clone(),
                ..Default::default()
            },
        }
    }
}

#[cfg(feature = "pyo3")]
#[pymodule(name = "zed_types")]
pub mod python_module {
    #[pymodule_export]
    use crate::RGBDSensors;
}
