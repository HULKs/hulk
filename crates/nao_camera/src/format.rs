use std::mem::zeroed;

use nix::{errno::Errno, ioctl_readwrite};
use thiserror::Error;

use crate::bindings::{v4l2_buf_type_V4L2_BUF_TYPE_VIDEO_CAPTURE, v4l2_format};

#[derive(Debug, Error)]
pub enum SetFormatError {
    #[error("failed to set format")]
    FormatNotSet { source: Errno },
}

pub fn set_format(
    file_descriptor: i32,
    width: u32,
    height: u32,
    pixel_format: u32,
) -> Result<(), SetFormatError> {
    let mut format: v4l2_format = unsafe { zeroed() };
    format.type_ = v4l2_buf_type_V4L2_BUF_TYPE_VIDEO_CAPTURE;
    format.fmt.pix.width = width;
    format.fmt.pix.height = height;
    format.fmt.pix.pixelformat = pixel_format;
    unsafe { vidioc_set_format(file_descriptor, &mut format as *mut _) }
        .map_err(|source| SetFormatError::FormatNotSet { source })?;

    Ok(())
}

ioctl_readwrite!(vidioc_set_format, b'V', 5, v4l2_format);
