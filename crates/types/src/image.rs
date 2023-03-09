use color_eyre::eyre::{self, Context};
use serde::{
    de::{self, Error},
    Deserialize, Deserializer, Serialize, Serializer,
};
use serialize_hierarchy::SerializeHierarchy;
use std::{
    collections::BTreeSet,
    fmt::{self, Debug, Formatter},
    mem::{size_of, ManuallyDrop},
    ops::Index,
    path::Path,
    sync::Arc,
};

use image::{
    codecs::jpeg::JpegEncoder, io::Reader, load_from_memory_with_format, ImageFormat, RgbImage,
};
use nalgebra::Point2;

use crate::{Rgb, YCbCr444};

use super::color::YCbCr422;

const SERIALIZATION_JPEG_QUALITY: u8 = 40;

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct NaoImage {
    width_422: u32,
    height: u32,
    buffer: Arc<Vec<YCbCr422>>,
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
                let rgb_image = match load_from_memory_with_format(&jpeg_buffer, ImageFormat::Jpeg)
                {
                    Ok(ok) => ok,
                    Err(error) => {
                        return Err(serialize_hierarchy::Error::DeserializationFailed(
                            <D as Deserializer>::Error::custom(error),
                        ))
                    }
                }
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

impl Debug for NaoImage {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        struct DebugBuffer {
            buffer_length: usize,
        }

        impl Debug for DebugBuffer {
            fn fmt(&self, formatter: &mut Formatter<'_>) -> Result<(), fmt::Error> {
                formatter.write_fmt(format_args!(
                    "[{} pixel{}]",
                    self.buffer_length,
                    match self.buffer_length {
                        0 => "s",
                        1 => "",
                        _ => "s...",
                    }
                ))
            }
        }

        formatter
            .debug_struct("Image")
            .field("width_422", &self.width_422)
            .field("height", &self.height)
            .field(
                "buffer",
                &DebugBuffer {
                    buffer_length: self.buffer.len(),
                },
            )
            .finish()
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

// impl<'de> Deserialize<'de> for NaoImage {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//     where
//         D: Deserializer<'de>,
//     {
//         enum Field {
//             Width422,
//             Height,
//             CompactBuffer,
//             ReadableBuffer,
//         }
//         const FIELDS: &[&str] = &["width_422", "height", "buffer"];

//         impl<'de> Deserialize<'de> for Field {
//             fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//             where
//                 D: Deserializer<'de>,
//             {
//                 struct FieldVisitor {
//                     is_human_readable: bool,
//                 }

//                 impl<'de> Visitor<'de> for FieldVisitor {
//                     type Value = Field;

//                     fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
//                         formatter.write_str("`width_422`, `height`, or `buffer`")
//                     }

//                     fn visit_str<E>(self, field: &str) -> std::result::Result<Self::Value, E>
//                     where
//                         E: de::Error,
//                     {
//                         match field {
//                             "width_422" => Ok(Field::Width422),
//                             "height" => Ok(Field::Height),
//                             "buffer" => Ok(if self.is_human_readable {
//                                 Field::ReadableBuffer
//                             } else {
//                                 Field::CompactBuffer
//                             }),
//                             _ => Err(de::Error::unknown_field(field, FIELDS)),
//                         }
//                     }
//                 }

//                 let is_human_readable = deserializer.is_human_readable();
//                 deserializer.deserialize_identifier(FieldVisitor { is_human_readable })
//             }
//         }

//         struct ImageVisitor {
//             is_human_readable: bool,
//         }

//         impl<'de> Visitor<'de> for ImageVisitor {
//             type Value = NaoImage;

//             fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
//                 formatter.write_str("struct Image")
//             }

//             fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
//             where
//                 A: SeqAccess<'de>,
//             {
//                 let width_422 = sequence
//                     .next_element()?
//                     .ok_or_else(|| de::Error::invalid_length(0, &self))?;
//                 let height = sequence
//                     .next_element()?
//                     .ok_or_else(|| de::Error::invalid_length(1, &self))?;
//                 let buffer = if self.is_human_readable {
//                     sequence
//                         .next_element()?
//                         .ok_or_else(|| de::Error::invalid_length(2, &self))?
//                 } else {
//                     let rgb_image_buffer: ByteBuf = sequence
//                         .next_element()?
//                         .ok_or_else(|| de::Error::invalid_length(2, &self))?;
//                     let rgb_image =
//                         load_from_memory_with_format(&rgb_image_buffer, ImageFormat::Jpeg)
//                             .map_err(de::Error::custom)?
//                             .into_rgb8();
//                     Arc::new(buffer_422_from_rgb_image(rgb_image))
//                 };

//                 Ok(NaoImage {
//                     width_422,
//                     height,
//                     buffer,
//                 })
//             }

//             fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
//             where
//                 A: MapAccess<'de>,
//             {
//                 let mut width_422 = None;
//                 let mut height = None;
//                 let mut buffer = None;

//                 while let Some(key) = map.next_key()? {
//                     match key {
//                         Field::Width422 => {
//                             if width_422.is_some() {
//                                 return Err(de::Error::duplicate_field("width_422"));
//                             }
//                             width_422 = Some(map.next_value()?);
//                         }
//                         Field::Height => {
//                             if height.is_some() {
//                                 return Err(de::Error::duplicate_field("height"));
//                             }
//                             height = Some(map.next_value()?);
//                         }
//                         Field::CompactBuffer => {
//                             if buffer.is_some() {
//                                 return Err(de::Error::duplicate_field("buffer"));
//                             }
//                             let rgb_image_buffer: ByteBuf = map.next_value()?;
//                             let rgb_image =
//                                 load_from_memory_with_format(&rgb_image_buffer, ImageFormat::Jpeg)
//                                     .map_err(de::Error::custom)?
//                                     .into_rgb8();
//                             buffer = Some(Arc::new(buffer_422_from_rgb_image(rgb_image)));
//                         }
//                         Field::ReadableBuffer => {
//                             if buffer.is_some() {
//                                 return Err(de::Error::duplicate_field("buffer"));
//                             }
//                             buffer = Some(map.next_value()?);
//                         }
//                     }
//                 }

//                 let width_422 = width_422.ok_or_else(|| de::Error::missing_field("width_422"))?;
//                 let height = height.ok_or_else(|| de::Error::missing_field("height"))?;
//                 let buffer = buffer.ok_or_else(|| de::Error::missing_field("buffer"))?;

//                 Ok(NaoImage {
//                     width_422,
//                     height,
//                     buffer,
//                 })
//             }
//         }

//         let is_human_readable = deserializer.is_human_readable();
//         deserializer.deserialize_struct("Image", FIELDS, ImageVisitor { is_human_readable })
//     }
// }

#[cfg(test)]
mod tests {
    use std::mem::transmute;

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
        assert_eq!(image.at(1, 1), YCbCr444 { y: 1, cb: 2, cr: 4 });
    }

    #[derive(Debug, Deserialize, Serialize)]
    struct ImageTestingWrapper(NaoImage);

    impl PartialEq for ImageTestingWrapper {
        fn eq(&self, other: &Self) -> bool {
            let buffers_are_equal = if other.0.buffer.len() == 1
                && other.0.buffer[0]
                    == (YCbCr422 {
                        y1: 63,
                        cb: 127,
                        y2: 191,
                        cr: 255,
                    }) {
                // special case for test `compact_image_serialization` because of JPEG and YCbCr conversion losses
                self.0.buffer.len() == 1
                    && self.0.buffer[0]
                        == YCbCr422 {
                            y1: 75,
                            cb: 129,
                            y2: 151,
                            cr: 216,
                        }
            } else {
                self.0.buffer == other.0.buffer
            };
            self.0.width_422 == other.0.width_422
                && self.0.height == other.0.height
                && buffers_are_equal
        }
    }

    #[test]
    fn compact_image_serialization() {
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
        let rgb_image =
            rgb_image_from_buffer_422(image.0.width_422, image.0.height, &image.0.buffer);
        let mut rgb_image_buffer = vec![];
        {
            let mut encoder =
                JpegEncoder::new_with_quality(&mut rgb_image_buffer, SERIALIZATION_JPEG_QUALITY);
            encoder
                .encode_image(&rgb_image)
                .expect("failed to encode image");
        }
        // serde_test::Token requires static lifetime and since the byte slice is not used anymore once leaving the test, it should be safe (tm)
        let static_rgb_image_buffer: &'static [u8] =
            unsafe { transmute(rgb_image_buffer.as_slice()) };

        assert_tokens(
            &image.compact(),
            &[
                Token::NewtypeStruct {
                    name: "ImageTestingWrapper",
                },
                Token::Struct {
                    name: "Image",
                    len: 3,
                },
                Token::Str("width_422"),
                Token::U32(1),
                Token::Str("height"),
                Token::U32(1),
                Token::Str("buffer"),
                Token::Bytes(static_rgb_image_buffer),
                Token::StructEnd,
            ],
        );
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
                    name: "Image",
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
