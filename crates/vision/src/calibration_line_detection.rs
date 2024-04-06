use calibration::measurement::Measurement;
use color_eyre::{eyre::Ok, Result};
use context_attribute::context;
use framework::MainOutput;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use types::{
    camera_matrix::CameraMatrix,
    camera_position::CameraPosition,
    world_state::{CalibrationPhase, CalibrationState},
    ycbcr422_image::YCbCr422Image,
};

#[context]
pub struct CreationContext {
    // TODO parameters
}

#[context]
pub struct CycleContext {
    camera_matrix: RequiredInput<Option<CameraMatrix>, "camera_matrix?">,
    image: Input<YCbCr422Image, "image">,
    camera_position: Parameter<CameraPosition, "image_receiver.$cycler_instance.camera_position">,
    calibration_state: Input<CalibrationState, "control", "calibration_state">,
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
    // pub calibration_measurement: MainOutput<Option<SystemTime>>,
}

impl CalibrationLineDetection {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            last_capture_command_time: None,
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let measurement = match &context.calibration_state.phase {
            CalibrationPhase::CAPTURE { dispatch_time } => {
                let new_command =
                    self.last_capture_command_time
                        .map_or(true, |last_capture_command_time| {
                            dispatch_time.start_time != last_capture_command_time
                        });
                // info!("Last command and current commands timestamps.");
                if new_command {
                    Some(Measurement::default())
                } else {
                    None
                }
            }
            _ => None,
        };
        Ok(MainOutputs {
            calibration_measurement: measurement.into(),
        })
    }
}
