use std::{
    fmt::Debug,
    mem::{ManuallyDrop, size_of},
    path::Path,
    sync::Arc,
};

use color_eyre::eyre::{self, WrapErr};
use geometry::circle::Circle;
use image::{ImageError, ImageReader, RgbImage, error::DecodingError};
use num_traits::Euclid;
use serde::{Deserialize, Serialize};
use yuv::{
    YuvBiPlanarImage, YuvConversionMode, YuvRange, YuvStandardMatrix, bgr_to_rgb, yuv_nv12_to_rgb,
};
use zenoh_buffers::buffer::SplitBuffer;

use coordinate_systems::Pixel;
use linear_algebra::{Point2, vector};
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};

use crate::{
    color::{Rgb, YCbCr422, YCbCr444},
    jpeg::JpegImage,
};
use ros_z_msgs::sensor_msgs::Image as Ros2Image;

pub const SAMPLE_SIZE: usize = 32;

#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PathSerialize, PathIntrospect, PathDeserialize,
)]
#[path_serde(add_leaf(jpeg: JpegImage))]
pub struct YCbCr422Image {
    width_422: u32,
    height: u32,
    #[path_serde(leaf)]
    buffer: Arc<Vec<YCbCr422>>,
}

impl From<&RgbImage> for YCbCr422Image {
    fn from(rgb_image: &RgbImage) -> Self {
        let width_422 = rgb_image.width() / 2;
        let height = rgb_image.height();
        let data = rgb_image
            .to_vec()
            .chunks(6)
            .map(|pixel| {
                let left_color: YCbCr444 = Rgb {
                    red: pixel[0],
                    green: pixel[1],
                    blue: pixel[2],
                }
                .into();
                let right_color: YCbCr444 = Rgb {
                    red: pixel[3],
                    green: pixel[4],
                    blue: pixel[5],
                }
                .into();
                [left_color, right_color].into()
            })
            .collect();

        Self {
            width_422,
            height,
            buffer: Arc::new(data),
        }
    }
}

impl From<&YCbCr422Image> for RgbImage {
    fn from(ycbcr422_image: &YCbCr422Image) -> Self {
        let width_422 = ycbcr422_image.width_422;
        let height = ycbcr422_image.height;
        let buffer: &[YCbCr422] = &ycbcr422_image.buffer;
        let mut rgb_image = Self::new(2 * width_422, height);

        for y in 0..height {
            for x in 0..width_422 {
                let pixel = buffer[(y * width_422 + x) as usize];
                let left_color: Rgb = YCbCr444 {
                    y: pixel.y1,
                    cb: pixel.cb,
                    cr: pixel.cr,
                }
                .into();
                let right_color: Rgb = YCbCr444 {
                    y: pixel.y2,
                    cb: pixel.cb,
                    cr: pixel.cr,
                }
                .into();
                rgb_image.put_pixel(
                    x * 2,
                    y,
                    image::Rgb([left_color.red, left_color.green, left_color.blue]),
                );
                rgb_image.put_pixel(
                    x * 2 + 1,
                    y,
                    image::Rgb([right_color.red, right_color.green, right_color.blue]),
                );
            }
        }

        rgb_image
    }
}

impl From<YCbCr422Image> for RgbImage {
    fn from(ycbcr422_image: YCbCr422Image) -> Self {
        Self::from(&ycbcr422_image)
    }
}

impl TryFrom<&Ros2Image> for YCbCr422Image {
    type Error = ImageError;

    fn try_from(ros2_image: &Ros2Image) -> Result<Self, ImageError> {
        let image_data = ros2_image.data.contiguous();
        let width_422 = ros2_image.width / 2;
        let height = ros2_image.height;

        let data: Vec<YCbCr422> = match ros2_image.encoding.as_str() {
            "rgb8" => {
                let expected_len = (ros2_image.step * ros2_image.height) as usize;
                let row_pixel_bytes = (ros2_image.width * 3) as usize;

                if image_data.len() != expected_len || row_pixel_bytes > ros2_image.step as usize {
                    return Err(ImageError::Decoding(DecodingError::from_format_hint(
                        image::error::ImageFormatHint::Name(
                            "rgb8: Source buffer size does not match dimensions".to_string(),
                        ),
                    )));
                }

                image_data
                    .chunks_exact(ros2_image.step as usize)
                    .flat_map(|row_bytes| row_bytes[..row_pixel_bytes].chunks_exact(6))
                    .map(|pixel| {
                        let left_color: YCbCr444 = Rgb {
                            red: pixel[0],
                            green: pixel[1],
                            blue: pixel[2],
                        }
                        .into();
                        let right_color: YCbCr444 = Rgb {
                            red: pixel[3],
                            green: pixel[4],
                            blue: pixel[5],
                        }
                        .into();
                        [left_color, right_color].into()
                    })
                    .collect()
            }
            "nv12" => {
                let y_plane_size = (ros2_image.width * ros2_image.height) as usize;
                let uv_plane_size = (ros2_image.width * ros2_image.height / 2) as usize;

                if image_data.len() < y_plane_size + uv_plane_size {
                    return Err(ImageError::Decoding(DecodingError::from_format_hint(
                        image::error::ImageFormatHint::Name(
                            "NV12: Source buffer is too small for the given dimensions".to_string(),
                        ),
                    )));
                }

                let y_stride = ros2_image.width;
                let chunked_y_stride = y_stride / 2;

                let (y_plane, uv_plane) = image_data.split_at(y_plane_size);

                assert_eq!(uv_plane.len(), uv_plane_size);

                y_plane
                    .chunks_exact(2)
                    .enumerate()
                    .map(|(i, y_chunk)| {
                        let (y_row, column) = (i as u32).div_rem_euclid(&chunked_y_stride);
                        let uv_row = y_row / 2;

                        assert!(uv_row <= ros2_image.height / 2);
                        assert!(column <= ros2_image.width / 2);

                        // don't forget sunscreen
                        let uv_index = uv_row * chunked_y_stride + column;
                        let uv_byte_index = uv_index as usize * 2;

                        assert!(uv_byte_index < uv_plane_size);

                        let cb = uv_plane[uv_byte_index];
                        let cr = uv_plane[uv_byte_index + 1];

                        YCbCr422 {
                            y1: y_chunk[0],
                            cb,
                            y2: y_chunk[1],
                            cr,
                        }
                    })
                    .collect()
            }
            encoding => unimplemented!(r#"image encoding "{encoding}" not supported"#),
        };

        Ok(Self {
            width_422,
            height,
            buffer: Arc::new(data),
        })
    }
}

pub fn rgb_image_from_ros_image(image: &Ros2Image) -> Result<RgbImage, ImageError> {
    let image_data = image.data.contiguous();

    match image.encoding.as_str() {
        "rgb8" => {
            let expected_len = (image.step * image.height) as usize;
            let row_pixel_bytes = (image.width * 3) as usize;

            if image_data.len() != expected_len || row_pixel_bytes > image.step as usize {
                return Err(ImageError::Decoding(DecodingError::from_format_hint(
                    image::error::ImageFormatHint::Name(
                        "rgb8: Source buffer size does not match dimensions".to_string(),
                    ),
                )));
            }

            let mut rgb_data = Vec::with_capacity(row_pixel_bytes * image.height as usize);
            for row_bytes in image_data.chunks_exact(image.step as usize) {
                rgb_data.extend_from_slice(&row_bytes[..row_pixel_bytes]);
            }

            RgbImage::from_raw(image.width, image.height, rgb_data).ok_or(ImageError::Decoding(
                DecodingError::from_format_hint(image::error::ImageFormatHint::Unknown),
            ))
        }
        "nv12" => {
            let y_plane_size = (image.step * image.height) as usize;
            let uv_plane_size = (image.step * image.height / 2) as usize;

            if image_data.len() != y_plane_size + uv_plane_size {
                return Err(ImageError::Decoding(DecodingError::from_format_hint(
                    image::error::ImageFormatHint::Name(
                        "NV12: Source buffer is too small for the given dimensions".to_string(),
                    ),
                )));
            }

            let mut rgb_image = RgbImage::new(image.width, image.height);
            let y_stride = image.step;
            let uv_stride = image.step;
            let rgb_stride = image.width * 3;

            let (y_plane, remaining) = image_data.split_at(y_plane_size);
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
                YuvRange::Limited,
                YuvStandardMatrix::Bt709,
                YuvConversionMode::Balanced,
            )
            .map_err(|error| {
                ImageError::Decoding(DecodingError::from_format_hint(
                    image::error::ImageFormatHint::Name(format!("NV12: {error}")),
                ))
            })?;

            Ok(rgb_image)
        }
        "bgr8" => {
            let expected_len = (image.step * image.height) as usize;
            let row_pixel_bytes = (image.width * 3) as usize;

            if image_data.len() != expected_len || row_pixel_bytes > image.step as usize {
                return Err(ImageError::Decoding(DecodingError::from_format_hint(
                    image::error::ImageFormatHint::Name(
                        "bgr8: Source buffer size does not match dimensions".to_string(),
                    ),
                )));
            }

            let mut rgb_image = RgbImage::new(image.width, image.height);

            bgr_to_rgb(
                image_data.as_ref(),
                image.step,
                rgb_image.as_flat_samples_mut().as_mut_slice(),
                image.width * 3,
                image.width,
                image.height,
            )
            .map_err(|error| {
                ImageError::Decoding(DecodingError::from_format_hint(
                    image::error::ImageFormatHint::Name(format!("bgr8: {error}")),
                ))
            })?;

            Ok(rgb_image)
        }
        "mono16" => {
            let expected_len = (image.step * image.height) as usize;
            let row_pixel_bytes = (image.width * 2) as usize;

            if image_data.len() != expected_len
                || !image_data.len().is_multiple_of(2)
                || row_pixel_bytes > image.step as usize
            {
                return Err(ImageError::Decoding(DecodingError::from_format_hint(
                    image::error::ImageFormatHint::Name(
                        "mono16: Source buffer size does not match dimensions".to_string(),
                    ),
                )));
            }

            let mut rgb_image = RgbImage::new(image.width, image.height);
            let output_buffer = rgb_image.as_mut();

            for (row_index, row_bytes) in image_data.chunks_exact(image.step as usize).enumerate() {
                for (column_index, bytes) in
                    row_bytes[..row_pixel_bytes].chunks_exact(2).enumerate()
                {
                    let pixel_val = if image.is_bigendian != 0 {
                        u16::from_be_bytes([bytes[0], bytes[1]])
                    } else {
                        u16::from_le_bytes([bytes[0], bytes[1]])
                    };
                    let gray_8 = (pixel_val >> 8) as u8;
                    let rgb_idx = (row_index * image.width as usize + column_index) * 3;
                    output_buffer[rgb_idx] = gray_8;
                    output_buffer[rgb_idx + 1] = gray_8;
                    output_buffer[rgb_idx + 2] = gray_8;
                }
            }

            Ok(rgb_image)
        }
        encoding => Err(ImageError::Decoding(DecodingError::from_format_hint(
            image::error::ImageFormatHint::Name(format!("unknown encoding: {encoding}")),
        ))),
    }
}

impl YCbCr422Image {
    pub fn zero(width: u32, height: u32) -> Self {
        assert!(
            width.is_multiple_of(2),
            "YCbCr422Image does not support odd widths because pixels are stored in pairs. Dimensions were {width}x{height}",
        );
        Self::from_ycbcr_buffer(
            width / 2,
            height,
            vec![YCbCr422::default(); width as usize / 2 * height as usize],
        )
    }

    pub fn from_ycbcr_buffer(width_422: u32, height: u32, buffer: Vec<YCbCr422>) -> Self {
        assert_eq!(buffer.len() as u32, width_422 * height);
        Self {
            width_422,
            height,
            buffer: Arc::new(buffer),
        }
    }

    pub fn from_raw_buffer(width_422: u32, height: u32, buffer: Vec<u8>) -> Self {
        let mut buffer = ManuallyDrop::new(buffer);

        let u8_pointer = buffer.as_mut_ptr();
        let u8_length = buffer.len();
        let u8_capacity = buffer.capacity();

        assert_eq!(u8_length % size_of::<YCbCr422>(), 0);
        assert_eq!(u8_capacity % size_of::<YCbCr422>(), 0);

        let ycbcr_pointer = u8_pointer as *mut YCbCr422;
        let ycbcr_length = u8_length / size_of::<YCbCr422>();
        let ycbcr_capacity = u8_capacity / size_of::<YCbCr422>();

        let buffer = unsafe { Vec::from_raw_parts(ycbcr_pointer, ycbcr_length, ycbcr_capacity) };

        Self {
            width_422,
            height,
            buffer: Arc::new(buffer),
        }
    }

    pub fn load_from_444_png(path: impl AsRef<Path>) -> eyre::Result<Self> {
        let png = ImageReader::open(path)?.decode()?.into_rgb8();

        let width = png.width();
        let height = png.height();
        let rgb_pixels = png.into_vec();

        let pixels = rgb_pixels
            .chunks(6)
            .map(|x| YCbCr422 {
                y1: x[0],
                cb: ((x[1] as u16 + x[4] as u16) / 2) as u8,
                y2: x[3],
                cr: ((x[2] as u16 + x[5] as u16) / 2) as u8,
            })
            .collect();

        Ok(Self::from_ycbcr_buffer(width / 2, height, pixels))
    }

    pub fn save_to_ycbcr_444_file(&self, file: impl AsRef<Path>) -> eyre::Result<()> {
        let mut image = RgbImage::new(2 * self.width_422, self.height);
        for y in 0..self.height {
            for x in 0..self.width_422 {
                let pixel = self.buffer[(y * self.width_422 + x) as usize];
                image.put_pixel(x * 2, y, image::Rgb([pixel.y1, pixel.cb, pixel.cr]));
                image.put_pixel(x * 2 + 1, y, image::Rgb([pixel.y2, pixel.cb, pixel.cr]));
            }
        }
        Ok(image.save(file)?)
    }

    pub fn load_from_rgb_file(path: impl AsRef<Path>) -> eyre::Result<Self> {
        let rgb_image = ImageReader::open(path)?.decode()?.into_rgb8();
        Ok(Self::from(&rgb_image))
    }

    pub fn save_to_rgb_file(&self, file: impl AsRef<Path> + Debug) -> eyre::Result<()> {
        RgbImage::from(self)
            .save(&file)
            .wrap_err_with(|| format!("failed to save image to {file:?}"))
    }

    pub fn width(&self) -> u32 {
        self.width_422 * 2
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    fn coordinates_to_buffer_index(&self, x: u32, y: u32) -> usize {
        let x_422 = x / 2;
        (y * self.width_422 + x_422) as usize
    }

    pub fn at(&self, x: u32, y: u32) -> YCbCr444 {
        let pixel = self.buffer[self.coordinates_to_buffer_index(x, y)];
        let is_left_pixel = x.is_multiple_of(2);
        YCbCr444 {
            y: if is_left_pixel { pixel.y1 } else { pixel.y2 },
            cb: pixel.cb,
            cr: pixel.cr,
        }
    }

    pub fn at_point(&self, point: Point2<Pixel, u32>) -> YCbCr444 {
        self.at(point.x(), point.y())
    }

    pub fn try_at(&self, x: u32, y: u32) -> Option<YCbCr444> {
        if x >= self.width() || y >= self.height() {
            return None;
        }
        let pixel = self.buffer[self.coordinates_to_buffer_index(x, y)];
        let is_left_pixel = x.is_multiple_of(2);
        let pixel = YCbCr444 {
            y: if is_left_pixel { pixel.y1 } else { pixel.y2 },
            cb: pixel.cb,
            cr: pixel.cr,
        };
        Some(pixel)
    }

    /// row-major
    pub fn iter_pixels(&self) -> impl Iterator<Item = YCbCr444> + '_ {
        self.buffer.iter().flat_map(|&ycbcr422| {
            let ycbcr444: [YCbCr444; 2] = ycbcr422.into();
            ycbcr444
        })
    }

    pub fn sample_grayscale(&self, candidate: Circle<Pixel>) -> Sample {
        let top_left = candidate.center - vector![candidate.radius, candidate.radius];
        let image_pixels_per_sample_pixel = candidate.radius * 2.0 / SAMPLE_SIZE as f32;

        let mut sample = Sample::default();
        for (y, column) in sample.iter_mut().enumerate() {
            for (x, pixel) in column.iter_mut().enumerate() {
                let x = (top_left.x() + x as f32 * image_pixels_per_sample_pixel) as u32;
                let y = (top_left.y() + y as f32 * image_pixels_per_sample_pixel) as u32;
                *pixel = self.try_at(x, y).map_or(128.0, |pixel| pixel.y as f32);
            }
        }

        sample
    }
}

pub type Sample = [[f32; SAMPLE_SIZE]; SAMPLE_SIZE];

#[cfg(test)]
mod tests {
    use image::{Rgb, RgbImage};
    use ros_z_msgs::sensor_msgs::Image;

    use super::{YCbCr422Image, rgb_image_from_ros_image};

    #[test]
    fn rgb8_ros_image_conversion_is_exact() {
        let ros_image = Image {
            width: 2,
            height: 1,
            encoding: "rgb8".to_string(),
            is_bigendian: 0,
            step: 6,
            data: vec![10, 20, 30, 40, 50, 60].into(),
            ..Default::default()
        };

        let rgb_image = rgb_image_from_ros_image(&ros_image).expect("failed to convert rgb8");

        assert_eq!(rgb_image.width(), 2);
        assert_eq!(rgb_image.height(), 1);
        assert_eq!(rgb_image.get_pixel(0, 0), &Rgb([10, 20, 30]));
        assert_eq!(rgb_image.get_pixel(1, 0), &Rgb([40, 50, 60]));
        assert_eq!(rgb_image.into_raw(), vec![10, 20, 30, 40, 50, 60]);
    }

    #[test]
    fn mono16_big_endian_ros_image_conversion_is_safe_and_correct() {
        let ros_image = Image {
            width: 2,
            height: 1,
            encoding: "mono16".to_string(),
            is_bigendian: 1,
            step: 4,
            data: vec![0x12, 0x34, 0xAB, 0xCD].into(),
            ..Default::default()
        };

        let rgb_image = rgb_image_from_ros_image(&ros_image).expect("failed to convert mono16");

        assert_eq!(
            rgb_image.into_raw(),
            vec![0x12, 0x12, 0x12, 0xAB, 0xAB, 0xAB]
        );
    }

    #[test]
    fn mono16_ros_image_conversion_ignores_padded_row_bytes() {
        let ros_image = Image {
            width: 1,
            height: 2,
            encoding: "mono16".to_string(),
            is_bigendian: 1,
            step: 4,
            data: vec![0x12, 0x34, 0xFF, 0xEE, 0xAB, 0xCD, 0xDD, 0xCC].into(),
            ..Default::default()
        };

        let rgb_image =
            rgb_image_from_ros_image(&ros_image).expect("failed to convert padded mono16");

        assert_eq!(rgb_image.width(), 1);
        assert_eq!(rgb_image.height(), 2);
        assert_eq!(
            rgb_image.into_raw(),
            vec![0x12, 0x12, 0x12, 0xAB, 0xAB, 0xAB]
        );
    }

    #[test]
    fn rgb8_ros_image_conversion_ignores_padded_row_bytes() {
        let ros_image = Image {
            width: 1,
            height: 2,
            encoding: "rgb8".to_string(),
            is_bigendian: 0,
            step: 4,
            data: vec![1, 2, 3, 255, 4, 5, 6, 254].into(),
            ..Default::default()
        };

        let rgb_image =
            rgb_image_from_ros_image(&ros_image).expect("failed to convert padded rgb8");

        assert_eq!(rgb_image.into_raw(), vec![1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn bgr8_ros_image_conversion_ignores_padded_row_bytes() {
        let ros_image = Image {
            width: 1,
            height: 2,
            encoding: "bgr8".to_string(),
            is_bigendian: 0,
            step: 4,
            data: vec![3, 2, 1, 255, 6, 5, 4, 254].into(),
            ..Default::default()
        };

        let rgb_image =
            rgb_image_from_ros_image(&ros_image).expect("failed to convert padded bgr8");

        assert_eq!(rgb_image.into_raw(), vec![1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn ycbcr422_ros_image_conversion_ignores_padded_rgb8_row_bytes() {
        let padded_ros_image = Image {
            width: 2,
            height: 2,
            encoding: "rgb8".to_string(),
            is_bigendian: 0,
            step: 8,
            data: vec![1, 2, 3, 4, 5, 6, 255, 254, 7, 8, 9, 10, 11, 12, 253, 252].into(),
            ..Default::default()
        };

        let tightly_packed_ros_image = Image {
            width: 2,
            height: 2,
            encoding: "rgb8".to_string(),
            is_bigendian: 0,
            step: 6,
            data: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12].into(),
            ..Default::default()
        };

        let padded_ycbcr = YCbCr422Image::try_from(&padded_ros_image)
            .expect("failed to convert padded rgb8 to ycbcr422");
        let tightly_packed_ycbcr = YCbCr422Image::try_from(&tightly_packed_ros_image)
            .expect("failed to convert tightly packed rgb8 to ycbcr422");

        assert_eq!(padded_ycbcr.width(), tightly_packed_ycbcr.width());
        assert_eq!(padded_ycbcr.height(), tightly_packed_ycbcr.height());
        assert_eq!(
            RgbImage::from(&padded_ycbcr),
            RgbImage::from(&tightly_packed_ycbcr)
        );
    }
}
