use color_eyre::{
    eyre::{eyre, Ok},
    Result,
};
use rand::Rng;
use serde::{Deserialize, Serialize};

use calibration::{lines::Lines, measurement::Measurement};
use context_attribute::context;
use coordinate_systems::{Ground, Pixel};
use framework::MainOutput;
use geometry::line::{Line, Line2};
use linear_algebra::point;
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
    calibration_command: Input<CalibrationCommand, "control", "calibration_command">,
    camera_position: Parameter<CameraPosition, "image_receiver.$cycler_instance.camera_position">,
    field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
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
        let calibration_measurement = if let CalibrationCommand::Capture {
            camera,
            dispatch_time,
        } = context.calibration_command
        {
            if camera == context.camera_position {
                let measurement = get_measurement_from_image(
                    context.image,
                    context.camera_matrix,
                    *context.camera_position,
                    context.field_dimensions,
                );

                Some(CalibrationCaptureResponse {
                    dispatch_time: *dispatch_time,
                    measurement: measurement.ok(),
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

fn get_measurement_from_image(
    image: &YCbCr422Image,
    matrix: &CameraMatrix,
    position: CameraPosition,
    field_dimensions: &FieldDimensions,
) -> Result<Measurement> {
    // TODO replace with a real implementation

    get_fake_measurement(image, matrix, position, field_dimensions)
}

fn project_line_to_camera(matrix: &CameraMatrix, line: Line2<Ground>) -> Result<Line2<Pixel>> {
    Ok(Line(
        matrix.ground_to_pixel(line.0)?,
        matrix.ground_to_pixel(line.1)?,
    ))
}

fn get_fake_measurement(
    _image: &YCbCr422Image,
    matrix: &CameraMatrix,
    position: CameraPosition,
    field_dimensions: &FieldDimensions,
) -> Result<Measurement> {
    // Minimal length lines representing the 3 lines to make sure they are in the camera's fi
    // otherwise occlusions/ trimmed lines have to be handled
    let border_line = Line(point![2.0, 0.0], point![3.0, 0.0]);
    let goal_box_line = {
        let y = field_dimensions.goal_box_area_length + border_line.0.y();
        let bottom_x = border_line.0.x() + 0.5;
        Line(point![bottom_x, y], point![bottom_x + 1.0, y])
    };
    let connecting_line = Line(
        goal_box_line.0,
        point![goal_box_line.0.x(), border_line.0.y()],
    );

    let mut rng = rand::thread_rng();
    if rng.gen_range(0..10) > 5 {
        Ok(Measurement {
            matrix: matrix.clone(),
            position,
            lines: Lines {
                border_line: project_line_to_camera(matrix, border_line)?,
                connecting_line: project_line_to_camera(matrix, connecting_line)?,
                goal_box_line: project_line_to_camera(matrix, goal_box_line)?,
            },
        })
    } else {
        Err(eyre!("don't have a measurement for you!"))
    }
}
