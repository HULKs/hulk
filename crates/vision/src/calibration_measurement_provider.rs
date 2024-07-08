use color_eyre::{
    eyre::{eyre, Ok},
    Result,
};
use rand::Rng;
use serde::{Deserialize, Serialize};

use calibration::center_circle::{circle_points::CenterCirclePoints, measurement::Measurement};
use context_attribute::context;
use coordinate_systems::{Ground, Pixel};
use framework::MainOutput;
use linear_algebra::{point, Point2};
use projection::{camera_matrix::CameraMatrix, Projection};
use types::{
    calibration::{CalibrationCaptureResponse, CalibrationCommand},
    camera_position::CameraPosition,
    field_dimensions::FieldDimensions,
    ycbcr422_image::YCbCr422Image,
};

#[derive(Deserialize, Serialize)]
pub struct CalibrationMeasurementProvider {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    camera_matrix: RequiredInput<Option<CameraMatrix>, "camera_matrix?">,
    image: Input<YCbCr422Image, "image">,
    calibration_command: Input<Option<CalibrationCommand>, "control", "calibration_command?">,
    camera_position: Parameter<CameraPosition, "image_receiver.$cycler_instance.camera_position">,
    field_dimensions: Parameter<FieldDimensions, "field_dimensions">,

    detected_circles:
        Input<Option<Vec<(Point2<Pixel>, Vec<Point2<Pixel>>)>>, "detected_calibration_circles?">,
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
        let calibration_measurement = if let Some(CalibrationCommand {
            camera,
            dispatch_time,
            capture,
            ..
        }) = context.calibration_command
        {
            if *capture && camera == context.camera_position {
                let measurement = context.detected_circles.and_then(|circles| {
                    circles.first().map(|(center, points)| Measurement {
                        circle_and_points: CenterCirclePoints {
                            center: center.clone(),
                            points: points.clone(),
                        },
                        position: *context.camera_position,
                        matrix: context.camera_matrix.clone(),
                    })
                });

                Some(CalibrationCaptureResponse {
                    dispatch_time: *dispatch_time,
                    measurement: measurement,
                })
            } else {
                None
            }
        } else {
            None
        };

        Ok(MainOutputs {
            calibration_measurement: calibration_measurement.into(),
        })
    }
}
