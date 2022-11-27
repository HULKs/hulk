use std::mem::zeroed;

use nix::{errno::Errno, ioctl_readwrite};
use thiserror::Error;

use crate::bindings::v4l2_control;

#[derive(Debug, Error)]
pub enum SetControlError {
    #[error("failed to set control")]
    ControlNotSet { source: Errno },
}

pub fn set_control(file_descriptor: i32, id: u32, value: i32) -> Result<(), SetControlError> {
    let mut control: v4l2_control = unsafe { zeroed() };
    control.id = id;
    control.value = value;
    unsafe { vidioc_set_control(file_descriptor, &mut control as *mut _) }
        .map_err(|source| SetControlError::ControlNotSet { source })?;

    Ok(())
}

ioctl_readwrite!(vidioc_set_control, b'V', 28, v4l2_control);
