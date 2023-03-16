use std::sync::Arc;

use image::{
    codecs::jpeg::JpegEncoder, load_from_memory_with_format, ImageBuffer, ImageError, ImageFormat,
    Luma,
};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::{DecodeJpeg, EncodeJpeg, SerializeHierarchy};

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
#[serialize_hierarchy(as_jpeg)]
pub struct GrayscaleImage {
    width: u32,
    height: u32,
    buffer: Arc<Vec<u8>>,
}

impl GrayscaleImage {
    pub fn from_vec(width: u32, height: u32, buffer: Vec<u8>) -> Self {
        Self {
            width,
            height,
            buffer: Arc::new(buffer),
        }
    }
}

impl EncodeJpeg for GrayscaleImage {
    const DEFAULT_QUALITY: u8 = 40;
    type Error = ImageError;

    fn encode_as_jpeg(&self, quality: u8) -> Result<Vec<u8>, Self::Error> {
        let gray_image = ImageBuffer::<Luma<u8>, &[u8]>::from_raw(
            self.width,
            self.height,
            self.buffer.as_slice(),
        )
        .unwrap();
        let mut jpeg_image_buffer = vec![];
        let mut encoder = JpegEncoder::new_with_quality(&mut jpeg_image_buffer, quality);
        encoder.encode_image(&gray_image)?;
        Ok(jpeg_image_buffer)
    }
}

impl DecodeJpeg for GrayscaleImage {
    type Error = ImageError;

    fn decode_from_jpeg(jpeg: Vec<u8>) -> Result<Self, Self::Error> {
        let luma_image = load_from_memory_with_format(&jpeg, ImageFormat::Jpeg)?.into_luma8();
        Ok(Self {
            width: luma_image.width(),
            height: luma_image.height(),
            buffer: luma_image.into_vec().into(),
        })
    }
}
