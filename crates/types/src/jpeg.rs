use image::{codecs::jpeg::JpegEncoder, ImageBuffer, ImageError, Luma, RgbImage};
use serde::{Deserialize, Serialize};

use crate::{grayscale_image::GrayscaleImage, ycbcr422_image::YCbCr422Image};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JpegImage {
    pub data: Vec<u8>,
}

impl TryFrom<&GrayscaleImage> for JpegImage {
    type Error = ImageError;

    fn try_from(image: &GrayscaleImage) -> Result<Self, Self::Error> {
        let gray_image =
            ImageBuffer::<Luma<u8>, &[u8]>::from_raw(image.width(), image.height(), image.buffer())
                .unwrap();
        let mut jpeg_buffer = vec![];
        let quality = 40;
        let mut encoder = JpegEncoder::new_with_quality(&mut jpeg_buffer, quality);
        encoder.encode_image(&gray_image)?;

        Ok(Self { data: jpeg_buffer })
    }
}

impl TryFrom<&YCbCr422Image> for JpegImage {
    type Error = ImageError;

    fn try_from(image: &YCbCr422Image) -> Result<Self, Self::Error> {
        let rgb_image = RgbImage::from(image);
        let mut jpeg_buffer = vec![];
        let quality = 40;
        let mut encoder = JpegEncoder::new_with_quality(&mut jpeg_buffer, quality);
        encoder.encode_image(&rgb_image)?;
        Ok(Self { data: jpeg_buffer })
    }
}
