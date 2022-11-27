use std::mem::zeroed;

use nix::{errno::Errno, ioctl_readwrite};
use thiserror::Error;

use crate::bindings::{uvc_xu_control_query, UVC_GET_CUR, UVC_SET_CUR};

#[derive(Debug, Error)]
pub enum UvcvideoError {
    #[error("failed to get uvc control")]
    ControlNotGet { source: Errno },
    #[error("failed to set uvc control")]
    ControlNotSet { source: Errno },
}

pub const UVC_EXTENSION_UNIT: u8 = 0x03;

pub fn set_control<const SIZE: usize>(
    file_descriptor: i32,
    unit: u8,
    selector: u8,
    data: &mut [u8; SIZE],
) -> Result<i32, UvcvideoError> {
    query_control(file_descriptor, unit, selector, UVC_SET_CUR as u8, data)
        .map_err(|source| UvcvideoError::ControlNotSet { source })
}

pub fn get_control<const SIZE: usize>(
    file_descriptor: i32,
    unit: u8,
    selector: u8,
    data: &mut [u8; SIZE],
) -> Result<i32, UvcvideoError> {
    query_control(file_descriptor, unit, selector, UVC_GET_CUR as u8, data)
        .map_err(|source| UvcvideoError::ControlNotGet { source })
}

fn query_control<const SIZE: usize>(
    file_descriptor: i32,
    unit: u8,
    selector: u8,
    get_or_set: u8,
    data: &mut [u8; SIZE],
) -> nix::Result<i32> {
    let mut query: uvc_xu_control_query = unsafe { zeroed() };
    query.unit = unit;
    query.selector = selector;
    query.query = get_or_set;
    query.size = SIZE as u16;
    query.data = data as *mut _;
    unsafe { uvc_query_control(file_descriptor, &mut query as *mut _) }
}

ioctl_readwrite!(uvc_query_control, b'u', 33, uvc_xu_control_query);
