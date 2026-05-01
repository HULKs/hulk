use std::time::Duration;

use ros_z_msgs::{
    builtin_interfaces::Time,
    sensor_msgs::{CameraInfo, Image, RegionOfInterest},
    std_msgs::Header,
};

pub const MAGIC_IDENTIFIER_FRAME: u32 = 0xC0FFEE42;
pub const MAGIC_IDENTIFIER_CAMERA_INFO: u32 = 0xCA11B42E;

#[repr(C, packed)]
#[derive(Debug, Clone)]
pub struct X5CameraFrameHeader {
    pub channel: u8,
    pub frame_identifier: u32,
    pub timestamp_nanoseconds: u64,
    pub width: u16,
    pub height: u16,
    pub payload_size: u32,
}

#[derive(Debug, Clone)]
pub struct X5CameraFrame {
    pub header: X5CameraFrameHeader,
    pub nv12_data: Vec<u8>,
}

impl From<X5CameraFrame> for Image {
    fn from(value: X5CameraFrame) -> Self {
        let timestamp = Duration::from_nanos(value.header.timestamp_nanoseconds);
        Image {
            header: Header {
                stamp: Time {
                    sec: timestamp.as_secs().min(i32::MAX as u64) as i32,
                    nanosec: timestamp.subsec_nanos(),
                },
                frame_id: "x5".to_owned(),
            },
            height: value.header.height.into(),
            width: value.header.width.into(),
            encoding: "nv12".to_owned(),
            is_bigendian: 0,
            step: value.header.width.into(),
            data: value.nv12_data.into(),
        }
    }
}

#[repr(C, packed)]
#[derive(Default, Clone, Copy)]
pub struct X5CameraInfo {
    pub width: u16,
    pub height: u16,
    pub distortion_model: [u8; 24],
    pub distortion_count_left: u8,
    pub distortion_count_right: u8,
    pub left_focal_length: [f64; 2],
    pub left_optical_center: [f64; 2],
    pub left_distortion_coefficients: [f64; 8],
    pub right_focal_length: [f64; 2],
    pub right_optical_center: [f64; 2],
    pub right_distortion_coefficients: [f64; 8],
    pub rotation_matrix: [f64; 9],
    pub translation_vector: [f64; 3],
    pub capture_width: u16,
    pub capture_height: u16,
    pub rectified_focal_length: [f64; 2],
    pub rectified_optical_center: [f64; 2],
    pub stereoscopic_baseline: f64,
    pub disparity_to_depth_matrix: [f64; 16],
}

impl X5CameraInfo {
    pub fn left_camera_info(&self) -> CameraInfo {
        let distortion_model = String::from_utf8_lossy(&self.distortion_model)
            .trim_matches('\0')
            .to_string();
        let left_distortion_coefficients = self.left_distortion_coefficients;

        CameraInfo {
            header: Header {
                stamp: Time { sec: 0, nanosec: 0 },
                frame_id: "x5".to_owned(),
            },
            height: self.height as u32,
            width: self.width as u32,
            distortion_model,
            d: left_distortion_coefficients[..self.distortion_count_left as usize].to_vec(),
            k: [
                self.left_focal_length[0],
                0.0,
                self.left_optical_center[0],
                0.0,
                self.left_focal_length[1],
                self.left_optical_center[1],
                0.0,
                0.0,
                1.0,
            ],
            // The X5 payload only transmits the relative right-to-left extrinsic rotation matrix,
            // not the individual stereo rectification matrices (R1/R2). Identity is used as the standard fallback.
            r: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
            p: [
                self.rectified_focal_length[0],
                0.0,
                self.rectified_optical_center[0],
                0.0,
                0.0,
                self.rectified_focal_length[1],
                self.rectified_optical_center[1],
                0.0,
                0.0,
                0.0,
                1.0,
                0.0,
            ],
            binning_x: 0,
            binning_y: 0,
            roi: RegionOfInterest::default(),
        }
    }

    pub fn right_camera_info(&self) -> CameraInfo {
        let distortion_model = String::from_utf8_lossy(&self.distortion_model)
            .trim_matches('\0')
            .to_string();
        let right_distortion_coefficients = self.right_distortion_coefficients;

        CameraInfo {
            header: Header {
                stamp: Time { sec: 0, nanosec: 0 },
                frame_id: "x5".to_owned(),
            },
            height: self.height as u32,
            width: self.width as u32,
            distortion_model,
            d: right_distortion_coefficients[..self.distortion_count_right as usize].to_vec(),
            k: [
                self.right_focal_length[0],
                0.0,
                self.right_optical_center[0],
                0.0,
                self.right_focal_length[1],
                self.right_optical_center[1],
                0.0,
                0.0,
                1.0,
            ],
            // The X5 payload only transmits the relative right-to-left extrinsic rotation matrix,
            // not the individual stereo rectification matrices (R1/R2). Identity is used as the standard fallback.
            r: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
            p: [
                self.rectified_focal_length[0],
                0.0,
                self.rectified_optical_center[0],
                -self.rectified_focal_length[0] * self.stereoscopic_baseline,
                0.0,
                self.rectified_focal_length[1],
                self.rectified_optical_center[1],
                0.0,
                0.0,
                0.0,
                1.0,
                0.0,
            ],
            binning_x: 0,
            binning_y: 0,
            roi: RegionOfInterest::default(),
        }
    }
}
