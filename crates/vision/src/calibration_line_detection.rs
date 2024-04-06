use std::time::SystemTime;

use calibration::measurement::Measurement;
use color_eyre::Result;
use context_attribute::context;
use framework::{AdditionalOutput, MainOutput};
use serde::{Deserialize, Serialize};
use types::{camera_matrix::CameraMatrix, ycbcr422_image::YCbCr422Image};

#[context]
pub struct CreationContext {
    // TODO parameters
}

#[context]
pub struct CycleContext {
    camera_matrix: RequiredInput<Option<CameraMatrix>, "camera_matrix?">,
    image: Input<YCbCr422Image, "image">,
    // capture_command: Input<SystemTime, "capture_command_time">,
    // TODO parameters
}

#[derive(Deserialize, Serialize)]
pub struct CalibrationLineDetection {
    // TODO add state as needed
    last_capture_command_time: Option<SystemTime>,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub calibration_measurement: MainOutput<Option<Measurement>>,
}

impl CalibrationLineDetection {
    pub fn new(context: CreationContext) -> Result<Self> {
        todo!()
    }
}
