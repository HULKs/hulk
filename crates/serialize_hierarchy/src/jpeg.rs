use image::ImageError;

pub const SERIALIZATION_JPEG_QUALITY: u8 = 40;

pub trait EncodeJpeg {
    fn encode_as_jpeg(&self, quality: u8) -> Result<Vec<u8>, ImageError>;
}

pub trait DecodeJpeg
where
    Self: Sized,
{
    fn decode_from_jpeg(jpeg: Vec<u8>) -> Result<Self, ImageError>;
}
