use std::{
    fmt::Debug,
    mem::{size_of, ManuallyDrop},
    ops::Index,
    path::Path,
    sync::Arc,
};

use color_eyre::eyre::{self, WrapErr};
use image::{io::Reader, RgbImage};
use serde::{Deserialize, Serialize};

use coordinate_systems::Pixel;
use linear_algebra::Point2;
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};

use crate::{
    color::{Rgb, YCbCr422, YCbCr444},
    jpeg::JpegImage,
};

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

impl From<RgbImage> for YCbCr422Image {
    fn from(rgb_image: RgbImage) -> Self {
        let width_422 = rgb_image.width() / 2;
        let height = rgb_image.height();
        let data = rgb_image
            .into_vec()
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

impl YCbCr422Image {
    pub fn zero(width: u32, height: u32) -> Self {
        assert!(
            width % 2 == 0,
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
        let rgb_image = Reader::open(path)?.decode()?.into_rgb8();
        Ok(Self::from(rgb_image))
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

    /// row-major
    pub fn iter_pixels(&self) -> impl Iterator<Item = YCbCr444> + '_ {
        self.buffer.iter().flat_map(|&ycbcr422| {
            let ycbcr444: [YCbCr444; 2] = ycbcr422.into();
            ycbcr444
        })
    }
}

impl Index<Point2<Pixel, usize>> for YCbCr422Image {
    type Output = YCbCr422;

    fn index(&self, position: Point2<Pixel, usize>) -> &Self::Output {
        &self.buffer[position.y() * self.width_422 as usize + position.x()]
    }
}
