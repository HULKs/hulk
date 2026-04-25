use crate::std_msgs::header::Header;
use color_eyre::{Result, eyre::eyre};
use image::{ImageError, RgbImage, error::DecodingError};
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};
use std::path::Path;
use yuv::{
    YuvBiPlanarImage, YuvConversionMode, YuvRange, YuvStandardMatrix, bgr_to_rgb, yuv_nv12_to_rgb,
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

    /// Subsamples an NV12 encoded image by exactly half in-place.
    /// Uses Nearest-Neighbor sampling to modify the internal data, width, height, and step.
    pub fn subsample_nv12_by_half_in_place(&mut self) -> Result<()> {
        if self.encoding != "nv12" {
            return Err(eyre!(
                "Subsampling currently only supported for nv12, got {}",
                self.encoding
            ));
        }

        let src_width = self.width as usize;
        let src_height = self.height as usize;
        let src_step = self.step as usize;

        if src_step < src_width {
            return Err(eyre!("Invalid NV12: step < width"));
        }

        if !src_width.is_multiple_of(4) || !src_height.is_multiple_of(4) {
            return Err(eyre!(
                "Width and height must be divisible by 4 for half NV12 subsampling"
            ));
        }

        let y_plane_size = src_step * src_height;
        let uv_plane_size = src_step * (src_height / 2);
        let expected_len = y_plane_size + uv_plane_size;

        if self.data.len() != expected_len {
            return Err(eyre!(
                "Invalid NV12 buffer size. Expected {}, got {}",
                expected_len,
                self.data.len()
            ));
        }

        let dest_width = src_width / 2;
        let dest_height = src_height / 2;
        let dest_step = dest_width;

        let dest_y_len = dest_width * dest_height;
        let dest_uv_len = dest_y_len / 2;

        let mut dest_data = vec![0u8; dest_y_len + dest_uv_len];

        let src = &self.data;
        let (dest_y, dest_uv) = dest_data.split_at_mut(dest_y_len);

        for y in 0..dest_height {
            let src_row = &src[(y * 2) * src_step..][..src_width];
            let dest_row = &mut dest_y[y * dest_width..][..dest_width];

            for (dx, sx) in dest_row.iter_mut().zip(src_row.iter().step_by(2)) {
                *dx = *sx;
            }
        }

        let src_uv = &src[y_plane_size..];
        let dest_uv_height = dest_height / 2;

        for y in 0..dest_uv_height {
            let src_row = &src_uv[(y * 2) * src_step..][..src_width];
            let dest_row = &mut dest_uv[y * dest_width..][..dest_width];

            for (dest_pair, src_pair) in dest_row.chunks_exact_mut(2).zip(src_row.chunks_exact(4)) {
                dest_pair[0] = src_pair[0];
                dest_pair[1] = src_pair[1];
            }
        }

        self.width = dest_width as u32;
        self.height = dest_height as u32;
        self.step = dest_step as u32;
        self.data = dest_data;

        Ok(())
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

                if image.data.len() != y_plane_size + uv_plane_size {
                    return Err(ImageError::Decoding(DecodingError::from_format_hint(
                        image::error::ImageFormatHint::Name(
                            "NV12: Source buffer is too small for the given dimensions".to_string(),
                        ),
                    )));
                }

                // RgbImage is a flattened Vec<u8> (R, G, B, R, G, B...)
                let mut rgb_image = RgbImage::new(image.width, image.height);

                // ROS 'step' is the stride for the Y plane.
                let y_stride = image.step;
                // NV12 UV plane usually has the same stride as Y
                let uv_stride = image.step;
                // RGB output stride (3 bytes per pixel * width)
                let rgb_stride = image.width * 3;

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
            "mono16" => {
                let pixel_count = (image.width * image.height) as usize;
                let u16_data: &[u16] = unsafe {
                    std::slice::from_raw_parts(image.data.as_ptr() as *const u16, pixel_count)
                };

                let mut rgb_image = RgbImage::new(image.width, image.height);
                let mut output_buffer = rgb_image.as_flat_samples_mut();
                let output_buffer = output_buffer.as_mut_slice();

                for (i, &pixel_val) in u16_data.iter().enumerate() {
                    let gray_8 = (pixel_val >> 8) as u8;

                    let rgb_idx = i * 3;
                    output_buffer[rgb_idx] = gray_8; // R
                    output_buffer[rgb_idx + 1] = gray_8; // G
                    output_buffer[rgb_idx + 2] = gray_8; // B
                }
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
