use color_eyre::eyre;
use image::{
    codecs::jpeg::JpegEncoder, ImageBuffer, ImageError, ImageFormat, ImageReader, ImageResult,
    Luma, RgbImage,
};
use ros2::sensor_msgs::image::Image;
use serde::{Deserialize, Serialize};
use std::{io, path::Path};

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
        let quality = 15;
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
        let quality = 15;
        let mut encoder = JpegEncoder::new_with_quality(&mut jpeg_buffer, quality);
        encoder.encode_image(&rgb_image)?;
        Ok(Self { data: jpeg_buffer })
    }
}

impl TryFrom<Image> for JpegImage {
    type Error = ImageError;

    fn try_from(image: Image) -> Result<Self, Self::Error> {
        let rgb_image = RgbImage::try_from(image)?;
        let mut jpeg_buffer = vec![];
        let quality = 15;
        let mut encoder = JpegEncoder::new_with_quality(&mut jpeg_buffer, quality);
        encoder.encode_image(&rgb_image)?;
        Ok(Self { data: jpeg_buffer })
    }
}

impl JpegImage {
    pub fn save_to_jpeg_file(&self, file: impl AsRef<Path>) -> eyre::Result<()> {
        Ok(std::fs::write(file, self.data.clone())?)
    }

    /// Returns width and height of the ´JpegImage´
    pub fn dimensions(&self) -> ImageResult<(u32, u32)> {
        ImageReader::with_format(io::Cursor::new(self.data.as_slice()), ImageFormat::Jpeg)
            .into_dimensions()
    }
}

#[cfg(test)]
mod tests {
    use super::{GrayscaleImage, JpegImage};

    #[test]
    fn jpeg_image_dimension() {
        let image = GrayscaleImage::from_vec(10, 20, vec![0; 200]);
        let jpeg_image = JpegImage::try_from(&image).expect("failed to convert grayscale to jpeg");
        assert_eq!(
            jpeg_image
                .dimensions()
                .expect("failed to get image dimensions"),
            (10, 20)
        );
    }
}
