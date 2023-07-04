use std::{
    fmt::Debug,
    mem::{size_of, ManuallyDrop},
    ops::Index,
    path::Path,
    sync::Arc,
};

use color_eyre::eyre::{self, WrapErr};
use image::{
    codecs::jpeg::JpegEncoder, io::Reader, load_from_memory_with_format, ImageError, ImageFormat,
    RgbImage,
};
use nalgebra::Point2;
use serde::{Deserialize, Serialize};
use serialize_hierarchy::{DecodeJpeg, EncodeJpeg, SerializeHierarchy};

use crate::{Rgb, YCbCr422, YCbCr444};

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
#[serialize_hierarchy(as_jpeg)]
pub struct YCbCr422Image {
    width_422: u32,
    height: u32,
    buffer: Arc<Vec<YCbCr422>>,
}

impl From<RgbImage> for YCbCr422Image {
    fn from(rgb_image: RgbImage) -> Self {
        Self {
            width_422: rgb_image.width() / 2,
            height: rgb_image.height(),
            buffer: Arc::new(buffer_422_from_rgb_image(rgb_image)),
        }
    }
}

impl From<&YCbCr422Image> for RgbImage {
    fn from(val: &YCbCr422Image) -> Self {
        rgb_image_from_buffer_422(val.width_422, val.height, &val.buffer)
    }
}

impl From<YCbCr422Image> for RgbImage {
    fn from(val: YCbCr422Image) -> Self {
        Self::from(&val)
    }
}

impl EncodeJpeg for YCbCr422Image {
    const DEFAULT_QUALITY: u8 = 40;
    type Error = ImageError;

    fn encode_as_jpeg(&self, quality: u8) -> Result<Vec<u8>, Self::Error> {
        let rgb_image: RgbImage = self.into();
        let mut jpeg_buffer = vec![];
        let mut encoder = JpegEncoder::new_with_quality(&mut jpeg_buffer, quality);
        encoder.encode_image(&rgb_image)?;
        Ok(jpeg_buffer)
    }
}

impl DecodeJpeg for YCbCr422Image {
    type Error = ImageError;

    fn decode_from_jpeg(jpeg: Vec<u8>) -> Result<Self, Self::Error> {
        let rgb_image = load_from_memory_with_format(&jpeg, ImageFormat::Jpeg)?.into_rgb8();
        Ok(rgb_image.into())
    }
}

impl YCbCr422Image {
    pub fn buffer(&self) -> &[YCbCr422] {
        &self.buffer
    }

    pub fn zero(width: u32, height: u32) -> Self {
        assert!(
            width % 2 == 0,
            "the Image type does not support odd widths. Dimensions were {width}x{height}",
        );
        Self::from_ycbcr_buffer(
            width / 2,
            height,
            vec![YCbCr422::default(); width as usize / 2 * height as usize],
        )
    }

    pub fn from_ycbcr_buffer(width_422: u32, height: u32, buffer: Vec<YCbCr422>) -> Self {
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
        let png = Reader::open(path)?.decode()?.into_rgb8();

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
        let png = Reader::open(path)?.decode()?.into_rgb8();

        let width = png.width();
        let height = png.height();

        let pixels = buffer_422_from_rgb_image(png);

        Ok(Self::from_ycbcr_buffer(width / 2, height, pixels))
    }

    pub fn save_to_rgb_file(&self, file: impl AsRef<Path> + Debug) -> eyre::Result<()> {
        rgb_image_from_buffer_422(self.width_422, self.height, &self.buffer)
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
        let is_left_pixel = x % 2 == 0;
        YCbCr444 {
            y: if is_left_pixel { pixel.y1 } else { pixel.y2 },
            cb: pixel.cb,
            cr: pixel.cr,
        }
    }

    pub fn try_at(&self, x: u32, y: u32) -> Option<YCbCr444> {
        if x >= self.width() || y >= self.height() {
            return None;
        }
        let pixel = self.buffer[self.coordinates_to_buffer_index(x, y)];
        let is_left_pixel = x % 2 == 0;
        let pixel = YCbCr444 {
            y: if is_left_pixel { pixel.y1 } else { pixel.y2 },
            cb: pixel.cb,
            cr: pixel.cr,
        };
        Some(pixel)
    }
}

impl Index<Point2<usize>> for YCbCr422Image {
    type Output = YCbCr422;

    fn index(&self, position: Point2<usize>) -> &Self::Output {
        &self.buffer[position.y * self.width_422 as usize + position.x]
    }
}

fn rgb_image_from_buffer_422(width_422: u32, height: u32, buffer: &[YCbCr422]) -> RgbImage {
    let mut rgb_image = RgbImage::new(2 * width_422, height);

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
                image::Rgb([left_color.r, left_color.g, left_color.b]),
            );
            rgb_image.put_pixel(
                x * 2 + 1,
                y,
                image::Rgb([right_color.r, right_color.g, right_color.b]),
            );
        }
    }

    rgb_image
}

fn buffer_422_from_rgb_image(rgb_image: RgbImage) -> Vec<YCbCr422> {
    rgb_image
        .into_vec()
        .chunks(6)
        .map(|pixel| {
            let left_color: YCbCr444 = Rgb {
                r: pixel[0],
                g: pixel[1],
                b: pixel[2],
            }
            .into();
            let right_color: YCbCr444 = Rgb {
                r: pixel[3],
                g: pixel[4],
                b: pixel[5],
            }
            .into();
            [left_color, right_color].into()
        })
        .collect()
}
