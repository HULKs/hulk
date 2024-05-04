use std::sync::Arc;

use path_serde::{PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

use crate::jpeg::JpegImage;

#[derive(Clone, Debug, Default, Deserialize, Serialize, PathSerialize, PathIntrospect)]
#[path_serde(add_leaf(jpeg: JpegImage))]
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

    pub fn buffer(&self) -> &[u8] {
        &self.buffer
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }
}
