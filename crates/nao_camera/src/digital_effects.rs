use thiserror::Error;

use crate::registers::{write_register, RegisterError};

#[derive(Debug, Error)]
pub enum DigitalEffectsError {
    #[error("failed to write register {address:#x} with value {value:#b}")]
    RegisterNotWritten {
        source: RegisterError,
        address: u16,
        value: u16,
    },
}

pub fn disable_digital_effects(file_descriptor: i32) -> Result<(), DigitalEffectsError> {
    // https://cdn.sparkfun.com/datasheets/Sensors/LightImaging/OV5640_datasheet.pdf, page 59
    // ISP CONTROL 01
    const ADDRESS: u16 = 0x5001;
    // Bit 0: Auto white balance: Enabled
    // Bit 1: Color matrix enable: Enabled
    // Bit 2: UV average enable: Disabled
    // Bit 5: Scaling enable: Enabled
    // Bit 7: Special Digital Effects enable: Disabled
    const VALUE: u16 = 0b00100011;
    write_register(file_descriptor, ADDRESS, VALUE).map_err(|source| {
        DigitalEffectsError::RegisterNotWritten {
            source,
            address: ADDRESS,
            value: VALUE,
        }
    })?;

    Ok(())
}
