pub trait EncodeJpeg {
    const DEFAULT_QUALITY: u8;
    type Error;
    fn encode_as_jpeg(&self, quality: u8) -> Result<Vec<u8>, Self::Error>;
}

pub trait DecodeJpeg
where
    Self: Sized,
{
    type Error;
    fn decode_from_jpeg(jpeg: Vec<u8>) -> Result<Self, Self::Error>;
}
