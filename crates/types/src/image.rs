use color_eyre::eyre::{self, Context};
use serde::{
    de::{self, Error},
    Deserialize, Deserializer, Serialize, Serializer,
};
use serialize_hierarchy::SerializeHierarchy;
use std::{
    collections::BTreeSet,
    fmt::Debug,
    mem::{size_of, ManuallyDrop},
    ops::Index,
    path::Path,
    sync::Arc,
};

use image::{
    codecs::jpeg::JpegEncoder, io::Reader, load_from_memory_with_format, ImageBuffer, ImageFormat,
    Luma, RgbImage,
};
use nalgebra::Point2;

use crate::{Rgb, YCbCr444};

use super::color::YCbCr422;

const SERIALIZATION_JPEG_QUALITY: u8 = 40;

#[derive(Clone, Default, Serialize, Deserialize, Debug)]
pub struct NaoImage {
    width_422: u32,
    height: u32,
    pub buffer: Arc<Vec<YCbCr422>>,
}

impl SerializeHierarchy for NaoImage {
    fn serialize_path<S>(
        &self,
        path: &str,
        serializer: S,
    ) -> Result<S::Ok, serialize_hierarchy::Error<S::Error>>
    where
        S: Serializer,
    {
        match path {
            "jpeg" => self
                .encode_as_jpeg()
                .serialize(serializer)
                .map_err(serialize_hierarchy::Error::SerializationFailed),
            _ => Err(serialize_hierarchy::Error::UnexpectedPathSegment {
                segment: path.to_string(),
            }),
        }
    }

    fn deserialize_path<'de, D>(
        &mut self,
        path: &str,
        deserializer: D,
    ) -> Result<(), serialize_hierarchy::Error<D::Error>>
    where
        D: Deserializer<'de>,
        <D as Deserializer<'de>>::Error: de::Error,
    {
        match path {
            "jpeg" => {
                let jpeg_buffer = Vec::<u8>::deserialize(deserializer)
                    .map_err(serialize_hierarchy::Error::DeserializationFailed)?;
                let rgb_image = load_from_memory_with_format(&jpeg_buffer, ImageFormat::Jpeg)
                    .map_err(|error| {
                        serialize_hierarchy::Error::DeserializationFailed(
                            <D as Deserializer>::Error::custom(error),
                        )
                    })?
                    .into_rgb8();
                self.width_422 = rgb_image.width() / 2;
                self.height = rgb_image.height() / 2;
                self.buffer = Arc::new(buffer_422_from_rgb_image(rgb_image));
                Ok(())
            }
            _ => Err(serialize_hierarchy::Error::UnexpectedPathSegment {
                segment: path.to_string(),
            }),
        }
    }

    fn exists(path: &str) -> bool {
        matches!(path, "raw" | "jpeg")
    }

    fn get_fields() -> BTreeSet<String> {
        ["jpeg".to_string()].into_iter().collect()
    }
}

impl NaoImage {
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

impl Index<Point2<usize>> for NaoImage {
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

trait EncodeJpeg {
    fn encode_as_jpeg(&self) -> Vec<u8>;
}

impl EncodeJpeg for NaoImage {
    fn encode_as_jpeg(&self) -> Vec<u8> {
        let rgb_image = rgb_image_from_buffer_422(self.width_422, self.height, &self.buffer);
        let mut jpeg_buffer = vec![];
        let mut encoder =
            JpegEncoder::new_with_quality(&mut jpeg_buffer, SERIALIZATION_JPEG_QUALITY);
        encoder
            .encode_image(&rgb_image)
            .expect("failed to encode image");
        jpeg_buffer
    }
}

#[derive(Clone, Default, Deserialize, Debug, SerializeHierarchy)]
pub struct YImage {
    width: u32,
    height: u32,
    buffer: Arc<Vec<u8>>,
}

impl YImage {
    pub fn from_vec(width: u32, height: u32, buffer: Vec<u8>) -> Self {
        Self {
            width,
            height,
            buffer: Arc::new(buffer),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_test::{assert_tokens, Configure, Token};
    use serialize_hierarchy::bincode::{deserialize, serialize};

    use super::*;

    #[test]
    fn image_has_zero_constructor() {
        let image = NaoImage::zero(10, 12);
        assert!(image.buffer.iter().all(|&x| x == YCbCr422::default()));
    }

    #[test]
    fn image_has_width_and_height() {
        let image = NaoImage::zero(10, 12);
        assert_eq!(image.width(), 10);
        assert_eq!(image.height(), 12);
    }

    #[test]
    fn image_can_be_indexed() {
        let image = NaoImage::from_ycbcr_buffer(
            2,
            2,
            vec![
                Default::default(),
                Default::default(),
                Default::default(),
                YCbCr422 {
                    y1: 1,
                    cb: 2,
                    y2: 3,
                    cr: 4,
                },
            ],
        );
        assert_eq!(image.at(2, 1), YCbCr444 { y: 1, cb: 2, cr: 4 });
    }

    #[derive(Debug, Deserialize, Serialize)]
    struct ImageTestingWrapper(NaoImage);

    impl PartialEq for ImageTestingWrapper {
        fn eq(&self, other: &Self) -> bool {
            let buffers_are_equal = self.0.buffer == other.0.buffer;
            self.0.width_422 == other.0.width_422
                && self.0.height == other.0.height
                && buffers_are_equal
        }
    }

    #[test]
    fn readable_image_serialization() {
        let image = ImageTestingWrapper(NaoImage {
            width_422: 1,
            height: 1,
            buffer: Arc::new(vec![YCbCr422 {
                y1: 0,
                cb: 1,
                y2: 2,
                cr: 3,
            }]),
        });

        assert_tokens(
            &image.readable(),
            &[
                Token::NewtypeStruct {
                    name: "ImageTestingWrapper",
                },
                Token::Struct {
                    name: "NaoImage",
                    len: 3,
                },
                Token::Str("width_422"),
                Token::U32(1),
                Token::Str("height"),
                Token::U32(1),
                Token::Str("buffer"),
                Token::Seq { len: Some(1) },
                Token::Struct {
                    name: "YCbCr422",
                    len: 4,
                },
                Token::Str("y1"),
                Token::U8(0),
                Token::Str("cb"),
                Token::U8(1),
                Token::Str("y2"),
                Token::U8(2),
                Token::Str("cr"),
                Token::U8(3),
                Token::StructEnd,
                Token::SeqEnd,
                Token::StructEnd,
            ],
        );
    }

    #[test]
    fn compact_serialization_and_deserialization_result_in_equality() {
        let image = ImageTestingWrapper(NaoImage {
            width_422: 1,
            height: 1,
            buffer: Arc::new(vec![YCbCr422 {
                y1: 63,
                cb: 127,
                y2: 191,
                cr: 255,
            }]),
        });

        let deserialized_serialized_image: ImageTestingWrapper =
            deserialize(&serialize(&image).unwrap()).unwrap();
        assert_eq!(deserialized_serialized_image, image);
    }
}
