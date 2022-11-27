use std::{thread::sleep, time::Duration};

use thiserror::Error;

use crate::uvcvideo::{get_control, set_control, UvcvideoError, UVC_EXTENSION_UNIT};

#[derive(Debug, Error)]
pub enum RegisterError {
    #[error("failed to get control")]
    ControlNotGet { source: UvcvideoError },
    #[error("failed to set control")]
    ControlNotSet { source: UvcvideoError },
}

const REGISTER_EXTENSION_UNIT_SELECTOR: u8 = 0x0e;
const REGISTER_READ: u8 = 0x00;
const REGISTER_WRITE: u8 = 0x01;

#[allow(dead_code)]
pub fn read_register(file_descriptor: i32, address: u16) -> Result<u16, RegisterError> {
    let upper_lower_address = address.to_be_bytes();

    let mut bytes = [
        REGISTER_READ,
        upper_lower_address[0],
        upper_lower_address[1],
        0,
        0,
    ];
    set_control(
        file_descriptor,
        UVC_EXTENSION_UNIT,
        REGISTER_EXTENSION_UNIT_SELECTOR,
        &mut bytes,
    )
    .map_err(|source| RegisterError::ControlNotSet { source })?;

    sleep(Duration::from_millis(10));

    get_control(
        file_descriptor,
        UVC_EXTENSION_UNIT,
        REGISTER_EXTENSION_UNIT_SELECTOR,
        &mut bytes,
    )
    .map_err(|source| RegisterError::ControlNotSet { source })?;

    Ok(u16::from_be_bytes([bytes[3], bytes[4]]))
}

pub fn write_register(
    file_descriptor: i32,
    address: u16,
    value: u16,
) -> Result<u16, RegisterError> {
    let upper_lower_address = address.to_be_bytes();
    let upper_lower_data = value.to_be_bytes();

    let mut bytes = [
        REGISTER_WRITE,
        upper_lower_address[0],
        upper_lower_address[1],
        upper_lower_data[0],
        upper_lower_data[1],
    ];
    set_control(
        file_descriptor,
        UVC_EXTENSION_UNIT,
        REGISTER_EXTENSION_UNIT_SELECTOR,
        &mut bytes,
    )
    .map_err(|source| RegisterError::ControlNotSet { source })?;

    Ok(u16::from_be_bytes([bytes[3], bytes[4]]))
}
