use calibration::measurement::Measurement;
use color_eyre::{eyre::Ok, Result};
use context_attribute::context;
use framework::MainOutput;
use projection::camera_matrix::CameraMatrix;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use types::{
    camera_position::CameraPosition,
    world_state::{CalibrationPhase, CalibrationState},
    ycbcr422_image::YCbCr422Image,
};

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    camera_matrix: RequiredInput<Option<CameraMatrix>, "camera_matrix?">,
    image: Input<YCbCr422Image, "image">,
    camera_position: Parameter<CameraPosition, "image_receiver.$cycler_instance.camera_position">,
    calibration_state: Input<CalibrationState, "control", "calibration_state">,
}

#[derive(Deserialize, Serialize)]
pub struct CalibrationMeasurementProvider {
    last_capture_command_time: Option<SystemTime>,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub calibration_measurement: MainOutput<Option<Measurement>>,
}

impl CalibrationMeasurementProvider {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            last_capture_command_time: None,
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let calibration_measurement = match &context.calibration_state.phase {
            CalibrationPhase::CAPTURE { dispatch_time } => {
                let new_request =
                    self.last_capture_command_time
                        .map_or(true, |last_capture_command_time| {
                            dispatch_time.start_time != last_capture_command_time
                        });
                if new_request {
                    get_measurement_from_image(
                        context.image,
                        context.camera_matrix,
                        *context.camera_position,
                    )
                } else {
                    None
                }
            }
            _ => None,
        }
        .into();
        Ok(MainOutputs {
            calibration_measurement,
        })
    }
}

fn get_measurement_from_image(
    image: &YCbCr422Image,
    matrix: &CameraMatrix,
    position: CameraPosition,
) -> Option<Measurement> {
    // TODO replace with a real implementation

    get_fake_measurement(image, matrix, position)
}

fn get_fake_measurement(
    _image: &YCbCr422Image,
    matrix: &CameraMatrix,
    position: CameraPosition,
) -> Option<Measurement> {
    let mut rng = rand::thread_rng();
    if rng.gen_range(0..10) > 8 {
        Some(Measurement {
            matrix: matrix.clone(),
            position,
            ..Default::default()
        })
    } else {
        None
    }
}
