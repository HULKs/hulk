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
    let address = 0x5001;
    let value = 0b00100011;
    write_register(file_descriptor, 0x5001, 0b00100011).map_err(|source| {
        DigitalEffectsError::RegisterNotWritten {
            source,
            address,
            value,
        }
    })?;

    Ok(())
}
