use crate::ycbcr422_image::YCbCr422Image;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SequenceNumber(u64);

impl Default for SequenceNumber {
    fn default() -> Self {
        Self::new(0)
    }
}

impl SequenceNumber {
    pub fn new(start_value: u64) -> Self {
        SequenceNumber(start_value)
    }
    pub fn next(&self) -> Self {
        Self(self.0 + 1)
    }
}

#[derive(Clone, Debug)]
pub struct CameraResult {
    pub sequence_number: SequenceNumber,
    pub image: YCbCr422Image,
}