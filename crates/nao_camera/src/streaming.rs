use nix::{errno::Errno, ioctl_write_buf};
use thiserror::Error;

use crate::bindings::v4l2_buf_type_V4L2_BUF_TYPE_VIDEO_CAPTURE;

#[derive(Debug, Error)]
pub enum StreamingError {
    #[error("failed to start stream")]
    StreamNotStarted { source: Errno },
    #[error("failed to stop stream")]
    StreamNotStopped { source: Errno },
}

pub fn stream_on(file_descriptor: i32) -> Result<(), StreamingError> {
    let type_ = v4l2_buf_type_V4L2_BUF_TYPE_VIDEO_CAPTURE;

    unsafe { vidioc_stream_on(file_descriptor, &[type_ as i32]) }
        .map_err(|source| StreamingError::StreamNotStarted { source })?;

    Ok(())
}

pub fn stream_off(file_descriptor: i32) -> Result<(), StreamingError> {
    let type_ = v4l2_buf_type_V4L2_BUF_TYPE_VIDEO_CAPTURE;

    unsafe { vidioc_stream_off(file_descriptor, &[type_ as i32]) }
        .map_err(|source| StreamingError::StreamNotStopped { source })?;

    Ok(())
}

ioctl_write_buf!(vidioc_stream_on, b'V', 18, i32);
ioctl_write_buf!(vidioc_stream_off, b'V', 19, i32);
