use std::{io, mem};

use nix::ioctl_readwrite;

pub use bindings::*;

pub mod bindings;

ioctl_readwrite!(uvc_query_control, b'u', 0x21, uvc_xu_control_query);

pub fn set_control<const SIZE: usize>(
    fd: std::os::raw::c_int,
    unit: u8,
    selector: u8,
    data: &[u8; SIZE],
) -> io::Result<[u8; SIZE]> {
    query_control(fd, unit, selector, UVC_SET_CUR as u8, data)
}

pub fn get_control<const SIZE: usize>(
    fd: std::os::raw::c_int,
    unit: u8,
    selector: u8,
    data: &[u8; SIZE],
) -> io::Result<[u8; SIZE]> {
    query_control(fd, unit, selector, UVC_GET_CUR as u8, data)
}

fn query_control<const SIZE: usize>(
    fd: std::os::raw::c_int,
    unit: u8,
    selector: u8,
    get_or_set: u8,
    data: &[u8; SIZE],
) -> io::Result<[u8; SIZE]> {
    let mut data = *data;
    unsafe {
        let mut xu: uvc_xu_control_query = mem::zeroed();
        xu.unit = unit;
        xu.selector = selector;
        xu.query = get_or_set;
        xu.size = SIZE as u16;
        xu.data = data.as_mut_ptr() as *mut u8;
        uvc_query_control(fd, &mut xu as *mut _).map_err(|_| io::Error::last_os_error())?;
    }
    Ok(data)
}
