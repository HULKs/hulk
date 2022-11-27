use std::mem::zeroed;

use nix::{errno::Errno, ioctl_readwrite};
use thiserror::Error;

use crate::bindings::{v4l2_buf_type_V4L2_BUF_TYPE_VIDEO_CAPTURE, v4l2_streamparm};

#[derive(Debug, Error)]
pub enum SetTimePerFrameError {
    #[error("failed to get parameters")]
    ParametersNotGot { source: Errno },
    #[error("failed to set parameters")]
    ParametersNotSet { source: Errno },
}

pub fn set_time_per_frame(
    file_descriptor: i32,
    numerator: u32,
    denominator: u32,
) -> Result<(), SetTimePerFrameError> {
    let mut parameters: v4l2_streamparm = unsafe { zeroed() };
    parameters.type_ = v4l2_buf_type_V4L2_BUF_TYPE_VIDEO_CAPTURE;
    unsafe { vidioc_get_parameters(file_descriptor, &mut parameters as *mut _) }
        .map_err(|source| SetTimePerFrameError::ParametersNotGot { source })?;

    parameters.parm.capture.timeperframe.numerator = numerator;
    parameters.parm.capture.timeperframe.denominator = denominator;
    unsafe { vidioc_set_parameters(file_descriptor, &mut parameters as *mut _) }
        .map_err(|source| SetTimePerFrameError::ParametersNotSet { source })?;

    Ok(())
}

ioctl_readwrite!(vidioc_get_parameters, b'V', 21, v4l2_streamparm);
ioctl_readwrite!(vidioc_set_parameters, b'V', 22, v4l2_streamparm);
