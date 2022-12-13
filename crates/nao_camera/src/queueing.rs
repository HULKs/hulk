use std::mem::zeroed;

use nix::{errno::Errno, ioctl_readwrite};
use thiserror::Error;

use crate::bindings::{
    v4l2_buf_type_V4L2_BUF_TYPE_VIDEO_CAPTURE, v4l2_buffer, v4l2_memory_V4L2_MEMORY_USERPTR,
};

#[derive(Debug, Error)]
pub enum QueueingError {
    #[error("failed to queue buffer")]
    BufferNotQueued { source: Errno },
    #[error("failed to dequeue buffer")]
    BufferNotDequeued { source: Errno },
}

pub fn queue(file_descriptor: i32, buffer_index: u32, buffer: &[u8]) -> Result<(), QueueingError> {
    let mut query: v4l2_buffer = unsafe { zeroed() };
    query.index = buffer_index;
    query.type_ = v4l2_buf_type_V4L2_BUF_TYPE_VIDEO_CAPTURE;
    query.memory = v4l2_memory_V4L2_MEMORY_USERPTR;
    query.m.userptr = buffer.as_ptr() as u64;
    query.length = buffer.len() as u32;

    unsafe { vidioc_queue(file_descriptor, &mut query as *mut _) }
        .map_err(|source| QueueingError::BufferNotQueued { source })?;

    Ok(())
}

pub fn dequeue(file_descriptor: i32) -> Result<u32, QueueingError> {
    let mut query: v4l2_buffer = unsafe { zeroed() };
    query.type_ = v4l2_buf_type_V4L2_BUF_TYPE_VIDEO_CAPTURE;
    query.memory = v4l2_memory_V4L2_MEMORY_USERPTR;

    unsafe { vidioc_dequeue(file_descriptor, &mut query as *mut _) }
        .map_err(|source| QueueingError::BufferNotDequeued { source })?;

    Ok(query.index)
}

ioctl_readwrite!(vidioc_queue, b'V', 15, v4l2_buffer);
ioctl_readwrite!(vidioc_dequeue, b'V', 17, v4l2_buffer);
