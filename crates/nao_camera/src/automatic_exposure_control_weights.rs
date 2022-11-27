use thiserror::Error;

use crate::uvcvideo::{get_control, set_control, UvcvideoError, UVC_EXTENSION_UNIT};

#[derive(Debug, Error)]
pub enum ExposureWeightsError {
    #[error("failed to get current control parameters")]
    ParametersNotGot { source: UvcvideoError },
    #[error("failed to set adjusted control parameters")]
    ParametersNotSet { source: UvcvideoError },
}

pub fn set_automatic_exposure_control_weights(
    file_descriptor: i32,
    weights: [u8; 16],
) -> Result<(), ExposureWeightsError> {
    assert!(weights.iter().all(|x| *x < 0x10));

    let mut bytes = [0; 17];
    get_control(file_descriptor, UVC_EXTENSION_UNIT, 0x09, &mut bytes)
        .map_err(|source| ExposureWeightsError::ParametersNotGot { source })?;

    for i in (0..weights.len()).step_by(2) {
        bytes[9 + i / 2] = (weights[i + 1] << 4) | weights[i];
    }

    set_control(file_descriptor, UVC_EXTENSION_UNIT, 0x09, &mut bytes)
        .map_err(|source| ExposureWeightsError::ParametersNotSet { source })?;

    Ok(())
}
