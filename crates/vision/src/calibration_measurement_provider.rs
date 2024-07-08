use color_eyre::{eyre::Ok, Result};
use serde::{Deserialize, Serialize};

use calibration::center_circle::{circle_points::CenterCirclePoints, measurement::Measurement};
use context_attribute::context;
use coordinate_systems::Pixel;
use framework::MainOutput;
use projection::camera_matrix::CameraMatrix;
use types::{
    calibration::{CalibrationCaptureResponse, CalibrationCommand},
    camera_position::CameraPosition,
};

#[derive(Deserialize, Serialize)]
pub struct CalibrationMeasurementProvider {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    camera_matrix: RequiredInput<Option<CameraMatrix>, "camera_matrix?">,
    calibration_command: Input<Option<CalibrationCommand>, "control", "calibration_command?">,
    camera_position: Parameter<CameraPosition, "image_receiver.$cycler_instance.camera_position">,

    calibration_center_circles:
        Input<Option<Vec<CenterCirclePoints<Pixel>>>, "calibration_center_circles?">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub calibration_measurement: MainOutput<Option<CalibrationCaptureResponse<Measurement>>>,
}

impl CalibrationMeasurementProvider {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let calibration_measurement = context.calibration_command.and_then(|command| {
            if !command.capture || command.camera != *context.camera_position {
                return None;
            }

            // If no output is found, the detection cycler was not run or early-exited
            let center_circles = context.calibration_center_circles?;

            // When the center_circles vector is empty -> no measurements found
            let measurement = center_circles
                .first()
                .map(|center_circle_points| Measurement {
                    circle_and_points: center_circle_points.clone(),
                    position: *context.camera_position,
                    matrix: context.camera_matrix.clone(),
                });

            Some(CalibrationCaptureResponse {
                dispatch_time: command.dispatch_time,
                measurement,
            })
        });

        Ok(MainOutputs {
            calibration_measurement: calibration_measurement.into(),
        })
    }
}
