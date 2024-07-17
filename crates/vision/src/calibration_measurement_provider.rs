use color_eyre::{eyre::Ok, Result};
use serde::{Deserialize, Serialize};

use calibration::center_circle::{circle_points::CenterCirclePoints, measurement::Measurement};
use context_attribute::context;
use coordinate_systems::Pixel;
use framework::MainOutput;
use projection::camera_matrix::CameraMatrix;
use types::{
    calibration::{
        CalibrationCaptureResponse, CalibrationCommand, CalibrationFeatureDetectorOutput,
    },
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

    calibration_center_circle: Input<
        CalibrationFeatureDetectorOutput<CenterCirclePoints<Pixel>>,
        "calibration_center_circle",
    >,
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

            if context.calibration_center_circle.cycle_skipped {
                return None;
            }

            let measurement = context
                .calibration_center_circle
                .detected_feature
                .clone()
                .map(|center_circle_points| Measurement {
                    circle_and_points: center_circle_points,
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
