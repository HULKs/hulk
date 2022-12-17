use std::{
    fmt::{Debug, Error, Formatter},
    mem::{size_of, ManuallyDrop},
    path::Path,
    sync::Arc,
    time::SystemTime,
};

use color_eyre::Result;
use image::{io::Reader, RgbImage};
use spl_network_messages::{GameControllerReturnMessage, GameControllerStateMessage, SplMessage};

use crate::{Rgb, YCbCr422, YCbCr444};

use super::{CameraPosition, Joints, Leds, SensorData};

pub trait Interface {
    fn read_from_microphones(&self) -> Result<Samples>;

    fn get_now(&self) -> SystemTime;
    fn get_ids(&self) -> Ids;
    fn read_from_sensors(&self) -> Result<SensorData>;
    fn write_to_actuators(&self, positions: Joints, stiffnesses: Joints, leds: Leds) -> Result<()>;

    fn read_from_network(&self) -> Result<IncomingMessage>;
    fn write_to_network(&self, message: OutgoingMessage) -> Result<()>;

    fn read_from_camera(&self, camera_position: CameraPosition) -> Result<Image>;
}

#[derive(Clone, Debug)]
pub struct Ids {
    pub body_id: String,
    pub head_id: String,
}

#[derive(Clone, Default)]
pub struct Image {
    buffer: Arc<Vec<YCbCr422>>,
    width_422: u32,
    height: u32,
}

impl Debug for Image {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result<(), Error> {
        formatter
            .debug_struct("Image")
            .field("buffer", &"...")
            .field("width_422", &self.width_422)
            .field("height", &self.height)
            .finish()
    }
}

impl Image {
    pub fn zero(width: u32, height: u32) -> Self {
        assert!(
            width % 2 == 0,
            "the Image type does not support odd widths. Dimensions were {width}x{height}",
        );
        Self::from_ycbcr_buffer(
            vec![YCbCr422::default(); width as usize / 2 * height as usize],
            width / 2,
            height,
        )
    }

    pub fn from_ycbcr_buffer(buffer: Vec<YCbCr422>, width_422: u32, height: u32) -> Self {
        Self {
            buffer: Arc::new(buffer),
            width_422,
            height,
        }
    }

    pub fn from_raw_buffer(buffer: Vec<u8>, width_422: u32, height: u32) -> Self {
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
            buffer: Arc::new(buffer),
            width_422,
            height,
        }
    }

    pub fn load_from_444_png(path: impl AsRef<Path>) -> Result<Self> {
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

        Ok(Self::from_ycbcr_buffer(pixels, width / 2, height))
    }

    pub fn save_to_ycbcr_444_file(&self, file: impl AsRef<Path>) -> Result<()> {
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

    pub fn save_to_rgb_file(&self, file: impl AsRef<Path>) -> Result<()> {
        let mut image = RgbImage::new(2 * self.width_422, self.height);
        for y in 0..self.height {
            for x in 0..self.width_422 {
                let pixel = self.buffer[(y * self.width_422 + x) as usize];
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
                image.put_pixel(
                    x * 2,
                    y,
                    image::Rgb([left_color.r, left_color.g, left_color.b]),
                );
                image.put_pixel(
                    x * 2 + 1,
                    y,
                    image::Rgb([right_color.r, right_color.g, right_color.b]),
                );
            }
        }
        Ok(image.save(file)?)
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

#[derive(Clone, Debug)]
pub enum IncomingMessage {
    GameController(GameControllerStateMessage),
    Spl(SplMessage),
}

impl Default for IncomingMessage {
    fn default() -> Self {
        IncomingMessage::GameController(Default::default())
    }
}

#[derive(Clone, Debug)]
pub enum OutgoingMessage {
    GameController(GameControllerReturnMessage),
    Spl(SplMessage),
}

impl Default for OutgoingMessage {
    fn default() -> Self {
        OutgoingMessage::GameController(Default::default())
    }
}

#[derive(Clone, Debug, Default)]
pub struct Samples {
    pub rate: u32,
    pub channels_of_samples: Arc<Vec<Vec<f32>>>,
}
