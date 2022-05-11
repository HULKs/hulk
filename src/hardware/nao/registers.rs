use std::{thread::sleep, time::Duration};

use anyhow::Context;
use log::trace;
use v4l::Device;

use crate::hardware::nao::nao_camera::UVC_EXTENSION_UNIT;

const REGISTER_EXTENSION_UNIT_SELECTOR: u8 = 0x0e;
const REGISTER_READ: u8 = 0x00;
const REGISTER_WRITE: u8 = 0x01;

#[allow(dead_code)]
pub fn read_register(device: &Device, address: u16) -> anyhow::Result<u16> {
    trace!("Reading camera register at address {:#04x}", address);
    let fd = device.handle().fd();
    let upper_lower_address = address.to_be_bytes();
    let bytes = [
        REGISTER_READ,
        upper_lower_address[0],
        upper_lower_address[1],
        0,
        0,
    ];
    uvcvideo::set_control(
        fd,
        UVC_EXTENSION_UNIT,
        REGISTER_EXTENSION_UNIT_SELECTOR,
        &bytes,
    )
    .context("Failed to set control for read register")?;
    sleep(Duration::from_millis(10));
    let control_data = uvcvideo::get_control(
        fd,
        UVC_EXTENSION_UNIT,
        REGISTER_EXTENSION_UNIT_SELECTOR,
        &bytes,
    )
    .context("Failed to get control for read register")?;
    let read_value = u16::from_be_bytes([control_data[3], control_data[4]]);
    Ok(read_value)
}

pub fn write_register(device: &Device, address: u16, value: u16) -> anyhow::Result<u16> {
    trace!(
        "Writing camera register at address {:#04x}, value {:#04x}",
        address,
        value
    );
    let fd = device.handle().fd();
    let upper_lower_address = address.to_be_bytes();
    let upper_lower_data = value.to_be_bytes();
    let bytes = [
        REGISTER_WRITE,
        upper_lower_address[0],
        upper_lower_address[1],
        upper_lower_data[0],
        upper_lower_data[1],
    ];
    let control_data = uvcvideo::set_control(
        fd,
        UVC_EXTENSION_UNIT,
        REGISTER_EXTENSION_UNIT_SELECTOR,
        &bytes,
    )
    .context("Failed to set control for read register")?;
    let written_data = u16::from_be_bytes([control_data[3], control_data[4]]);
    Ok(written_data)
}
