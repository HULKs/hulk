use crate::std_msgs::header::Header;
use color_eyre::Result;
use image::{error::DecodingError, ImageError, RgbImage};
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};
use std::path::Path;
use yuv::{
    bgr_to_rgb, yuv_nv12_to_rgb, YuvBiPlanarImage, YuvConversionMode, YuvRange, YuvStandardMatrix,
};

#[cfg(feature = "pyo3")]
use pyo3::{pyclass, pymethods};

/// This message contains an uncompressed image
/// (0, 0) is at top-left corner of image
#[cfg_attr(feature = "pyo3", pyclass(frozen))]
#[repr(C)]
#[derive(
    Clone, Debug, Default, Serialize, Deserialize, PathIntrospect, PathSerialize, PathDeserialize,
)]
pub struct Image {
    /// Header timestamp should be acquisition time of image
    /// Header frame_id should be optical frame of camera
    /// If the frame_id here and the frame_id of the CameraInfo
    /// message associated with the image conflict
    /// the behavior is undefined
    pub header: Header,

    /// origin of frame should be optical center of cameara
    /// +x should point to the right in the image
    /// +y should point down in the image
    /// +z should point into to plane of the image
    ///
    /// image height, that is, number of rows
    pub height: u32,
    /// image width, that is, number of columns
    pub width: u32,

    /// The legal values for encoding are in file src/image_encodings.cpp
    /// If you want to standardize a new string format, join
    /// ros-users@lists.ros.org and send an email proposing a new encoding.
    /// Encoding of pixels -- channel meaning, ordering, size
    /// taken from the list of strings in include/sensor_msgs/image_encodings.hpp
    pub encoding: String,

    /// is this data bigendian?
    pub is_bigendian: u8,
    /// Full row length in bytes
    pub step: u32,
    /// actual matrix data, size is (step * rows)
    pub data: Vec<u8>,
}

#[cfg(feature = "pyo3")]
#[pymethods]
impl Image {
    #[new]
    pub fn from_mujoco(time: f32, rgb: Vec<u8>, height: u32, width: u32) -> Self {
        use crate::builtin_interfaces::time::Time;
        use std::time::Duration;
        let simulation_duration = Duration::from_secs_f32(time);

        let header = Header {
            stamp: Time {
                sec: simulation_duration.as_secs() as i32,
                nanosec: simulation_duration.subsec_nanos(),
            },
            frame_id: "".to_string(),
        };

        Image {
            header: header.clone(),
            height,
            width,
            encoding: "rgb8".to_string(),
            is_bigendian: 0,
            step: width,
            data: rgb,
        }
    }
}

impl Image {
    pub fn save_to_file(self, file: impl AsRef<Path>) -> Result<()> {
        let rgb_image: RgbImage = self.try_into()?;
        Ok(rgb_image.save(file)?)
    }
}

impl TryFrom<Image> for RgbImage {
    type Error = ImageError;

    fn try_from(image: Image) -> Result<Self, ImageError> {
        match image.encoding.as_str() {
            "rgb8" => RgbImage::from_raw(image.width, image.height, image.data).ok_or(
                ImageError::Decoding(DecodingError::from_format_hint(
                    image::error::ImageFormatHint::Unknown,
                )),
            ),
            "nv12" => {
                let y_plane_size = (image.step * image.height) as usize;
                // UV plane is half height, but same stride as Y in NV12 (usually)
                let uv_plane_size = (image.step * image.height / 2) as usize;

                if image.data.len() < y_plane_size + uv_plane_size {
                    return Err(ImageError::Decoding(DecodingError::from_format_hint(
                        image::error::ImageFormatHint::Name(
                            "NV12: Source buffer is too small for the given dimensions".to_string(),
                        ),
                    )));
                }

                // 2. Prepare Output Buffer
                // RgbImage is a flattened Vec<u8> (R, G, B, R, G, B...)
                let mut rgb_image = RgbImage::new(image.width, image.height);

                // 3. Define Strides
                // ROS 'step' is the stride for the Y plane.
                let y_stride = image.step;
                // NV12 UV plane usually has the same stride as Y
                let uv_stride = image.step;
                // RGB output stride (3 bytes per pixel * width)
                let rgb_stride = image.width * 3;

                // 4. Split Input Data into Planes
                let (y_plane, remaining) = image.data.split_at(y_plane_size);
                let uv_plane = &remaining[..uv_plane_size];

                let yuv_bi_planar_image = YuvBiPlanarImage {
                    y_plane,
                    y_stride,
                    uv_plane,
                    uv_stride,
                    width: image.width,
                    height: image.height,
                };

                yuv_nv12_to_rgb(
                    &yuv_bi_planar_image,
                    rgb_image.as_flat_samples_mut().as_mut_slice(),
                    rgb_stride,
                    YuvRange::Limited, // Standard for video (16-235). Use 'Full' for JPEGs.
                    YuvStandardMatrix::Bt709, // Standard for HD Video. Use Bt601 for SD/Webcams.
                    YuvConversionMode::Balanced,
                )
                .map_err(|e| {
                    ImageError::Decoding(DecodingError::from_format_hint(
                        image::error::ImageFormatHint::Name(format!("NV12: {e}")),
                    ))
                })?;

                Ok(rgb_image)
            }
            "bgr8" => {
                let mut rgb_image = RgbImage::new(image.width, image.height);

                bgr_to_rgb(
                    &image.data,
                    image.step,
                    rgb_image.as_flat_samples_mut().as_mut_slice(),
                    image.step,
                    image.width,
                    image.height,
                )
                .map_err(|e| {
                    ImageError::Decoding(DecodingError::from_format_hint(
                        image::error::ImageFormatHint::Name(format!("bgr8: {e}")),
                    ))
                })?;

                Ok(rgb_image)
            }
            _ => Err(ImageError::Decoding(DecodingError::from_format_hint(
                image::error::ImageFormatHint::Name(format!(
                    "unknown encoding: {}",
                    image.encoding
                )),
            ))),
        }
    }
}
