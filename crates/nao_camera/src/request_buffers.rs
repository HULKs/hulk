use std::mem::zeroed;

use nix::{errno::Errno, ioctl_readwrite};
use thiserror::Error;

use crate::bindings::{
    v4l2_buf_type_V4L2_BUF_TYPE_VIDEO_CAPTURE, v4l2_memory_V4L2_MEMORY_USERPTR, v4l2_requestbuffers,
};

#[derive(Debug, Error)]
pub enum RequestBuffersError {
    #[error("failed to request buffers")]
    BuffersNotRequested { source: Errno },
}

pub fn request_user_pointer_buffers(
    file_descriptor: i32,
    amount_of_buffers: u32,
) -> Result<(), RequestBuffersError> {
    let mut query: v4l2_requestbuffers = unsafe { zeroed() };
    query.count = amount_of_buffers;
    query.type_ = v4l2_buf_type_V4L2_BUF_TYPE_VIDEO_CAPTURE;
    query.memory = v4l2_memory_V4L2_MEMORY_USERPTR;
    unsafe { vidioc_request_buffers(file_descriptor, &mut query as *mut _) }
        .map_err(|source| RequestBuffersError::BuffersNotRequested { source })?;

    Ok(())
}

ioctl_readwrite!(vidioc_request_buffers, b'V', 8, v4l2_requestbuffers);
