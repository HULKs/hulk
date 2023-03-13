use std::sync::Arc;

use image::{codecs::jpeg::JpegEncoder, ImageBuffer, Luma};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::{DecodeJpeg, EncodeJpeg, SerializeHierarchy};

#[derive(Clone, Default, Serialize, Deserialize, Debug, SerializeHierarchy)]
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
    fn encode_as_jpeg(&self, quality: u8) -> Result<Vec<u8>, image::ImageError> {
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
    fn decode_from_jpeg(jpeg: Vec<u8>) -> Result<Self, image::ImageError> {
        todo!()
    }
}

// impl Serialize for GrayscaleImage {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: Serializer,
//     {
//         let is_human_readable = serializer.is_human_readable();
//         let mut state = serializer.serialize_struct("Image", 3)?;
//         state.serialize_field("width", &self.width)?;
//         state.serialize_field("height", &self.height)?;
//         if is_human_readable {
//             state.serialize_field("buffer", &self.buffer)?;
//         } else {
//             let gray_image = ImageBuffer::<Luma<u8>, &[u8]>::from_raw(
//                 self.width,
//                 self.height,
//                 self.buffer.as_slice(),
//             )
//             .unwrap();
//             let mut jpeg_image_buffer = vec![];
//             {
//                 let mut encoder = JpegEncoder::new_with_quality(
//                     &mut jpeg_image_buffer,
//                     SERIALIZATION_JPEG_QUALITY,
//                 );
//                 encoder
//                     .encode_image(&gray_image)
//                     .expect("failed to encode image");
//             }
//             state.serialize_field("buffer", Bytes::new(jpeg_image_buffer.as_slice()))?;
//         }
//         state.end()
//     }
// }
