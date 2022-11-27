use thiserror::Error;

use crate::uvcvideo::{set_control, UvcvideoError, UVC_EXTENSION_UNIT};

#[derive(Debug, Error)]
pub enum FlipError {
    #[error("failed to flip horizontally")]
    NotFlippedHorizontally { source: UvcvideoError },
    #[error("failed to flip vertically")]
    NotFlippedVertically { source: UvcvideoError },
}

pub fn flip_sensor(file_descriptor: i32) -> Result<(), FlipError> {
    const HORIZONTAL_FLIP_UNIT_SELECTOR: u8 = 0x0c;
    set_control(
        file_descriptor,
        UVC_EXTENSION_UNIT,
        HORIZONTAL_FLIP_UNIT_SELECTOR,
        &mut [1, 0],
    )
    .map_err(|source| FlipError::NotFlippedHorizontally { source })?;

    const VERTICAL_FLIP_UNIT_SELECTOR: u8 = 0x0d;
    set_control(
        file_descriptor,
        UVC_EXTENSION_UNIT,
        VERTICAL_FLIP_UNIT_SELECTOR,
        &mut [1, 0],
    )
    .map_err(|source| FlipError::NotFlippedHorizontally { source })?;

    Ok(())
}
