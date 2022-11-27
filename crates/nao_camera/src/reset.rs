use std::{path::Path, thread::sleep, time::Duration};

use i2cdev::{
    core::I2CDevice,
    linux::{LinuxI2CDevice, LinuxI2CError},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ResetError {
    #[error("failed to create I2C device")]
    I2CDeviceNotCreated { source: LinuxI2CError },
    #[error("failed to read I2C data at {register:#x}")]
    I2CDataNotRead { source: LinuxI2CError, register: u8 },
    #[error("failed to write I2C data {value:#x} to {register:#x}")]
    I2CDataNotWritten {
        source: LinuxI2CError,
        value: u8,
        register: u8,
    },
    #[error("timeout while waiting for camera disconnect")]
    DisconnectTimeouted,
    #[error("timeout while waiting for camera connect")]
    ConnectTimeouted,
}

pub enum CameraPosition {
    Top,
    Bottom,
}

pub fn reset_camera_device(
    device_path: impl AsRef<Path>,
    camera_position: CameraPosition,
) -> Result<(), ResetError> {
    const SLAVE_ADDRESS: u16 = 0x41;
    let mut device = LinuxI2CDevice::new("/dev/i2c-head", SLAVE_ADDRESS)
        .map_err(|source| ResetError::I2CDeviceNotCreated { source })?;
    // make sure reset lines of the cameras are configured as outputs on GPIO chip
    if device
        .smbus_read_byte_data(0x3)
        .map_err(|source| ResetError::I2CDataNotRead {
            source,
            register: 0x3,
        })?
        & 0xc
        != 0x0
    {
        device
            .smbus_write_byte_data(0x1, 0x0)
            .map_err(|source| ResetError::I2CDataNotWritten {
                source,
                value: 0x0,
                register: 0x1,
            })?;
        device.smbus_write_byte_data(0x3, 0xf3).map_err(|source| {
            ResetError::I2CDataNotWritten {
                source,
                value: 0xf3,
                register: 0x3,
            }
        })?;
    }
    // GPIO pin layout:
    // - pin 0 (usb hub reset): input
    // - pin 1 (reserved):      input
    // - pin 2 (CX3 top camera reset): output (high -> running)
    // - pin 3 (CX3 bottom camera reset): output (high -> running)
    let i2c_set_value = match camera_position {
        CameraPosition::Top => 0x8,
        CameraPosition::Bottom => 0x4,
    };
    // disconnect this camera
    device
        .smbus_write_byte_data(0x1, i2c_set_value)
        .map_err(|source| ResetError::I2CDataNotWritten {
            source,
            value: i2c_set_value,
            register: 0x1,
        })?;
    poll_condition(
        || !device_path.as_ref().exists(),
        20,
        Duration::from_millis(50),
        ResetError::DisconnectTimeouted,
    )?;
    // connect both cameras (if not already connected)
    device
        .smbus_write_byte_data(0x1, 0xc)
        .map_err(|source| ResetError::I2CDataNotWritten {
            source,
            value: 0xc,
            register: 0x1,
        })?;
    poll_condition(
        || device_path.as_ref().exists(),
        40,
        Duration::from_millis(50),
        ResetError::ConnectTimeouted,
    )?;
    Ok(())
}

fn poll_condition(
    condition: impl Fn() -> bool,
    maximum_iteration_count: usize,
    sleep_time: Duration,
    error: ResetError,
) -> Result<(), ResetError> {
    for _ in 0..maximum_iteration_count {
        if condition() {
            return Ok(());
        }
        sleep(sleep_time);
    }
    Err(error)
}
