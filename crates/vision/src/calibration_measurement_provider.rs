use calibration::center_circle::{circles::CenterOfCircleAndPoints, measurement::Measurement};
use color_eyre::{
    eyre::{eyre, Ok},
    Result,
};
use context_attribute::context;
use coordinate_systems::{Ground, Pixel};
use framework::MainOutput;
use itertools::Itertools;
use linear_algebra::{point, Point2};
use projection::{camera_matrix::CameraMatrix, Projection};
use rand::{
    distributions::{Distribution, Uniform},
    Rng,
};
use serde::{Deserialize, Serialize};
use std::{f32::consts::PI, time::SystemTime};
use types::{
    camera_position::CameraPosition, field_dimensions::FieldDimensions,
    world_state::CalibrationCommand, ycbcr422_image::YCbCr422Image,
};

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    camera_matrix: RequiredInput<Option<CameraMatrix>, "camera_matrix?">,
    image: Input<YCbCr422Image, "image">,
    camera_position: Parameter<CameraPosition, "image_receiver.$cycler_instance.camera_position">,
    calibration_command: Input<CalibrationCommand, "control", "calibration_command">,
    field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
}

#[derive(Deserialize, Serialize)]
pub struct CalibrationMeasurementProvider {
    last_completed_capture_command_time: Option<SystemTime>,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub calibration_measurement: MainOutput<Option<Measurement>>,
}

impl CalibrationMeasurementProvider {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            last_completed_capture_command_time: None,
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let calibration_measurement = match &context.calibration_command {
            CalibrationCommand::CAPTURE { dispatch_time } => {
                let new_request = self.last_completed_capture_command_time.map_or(
                    true,
                    |last_capture_command_time| {
                        dispatch_time.start_time != last_capture_command_time
                    },
                );
                if new_request {
                    self.last_completed_capture_command_time = Some(dispatch_time.start_time);
                    get_measurement_from_image(
                        context.image,
                        context.camera_matrix,
                        *context.camera_position,
                        context.field_dimensions,
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
    field_dimensions: &FieldDimensions,
) -> Option<Measurement> {
    // TODO replace with a real implementation

    get_fake_measurement(image, matrix, position, field_dimensions).ok()
}

fn get_fake_measurement(
    _image: &YCbCr422Image,
    matrix: &CameraMatrix,
    position: CameraPosition,
    field_dimensions: &FieldDimensions,
) -> Result<Measurement> {
    const CIRCLE_CENTER_GROUND: Point2<Ground> = point![1.5, 0.3];
    const POINTS_PER_CIRCLE: usize = 20;
    const RADIUS_VARIANCE: f32 = 0.1;

    let radius = field_dimensions.center_circle_diameter / 2.0;
    let mut rng = rand::thread_rng();

    let circle_points: Vec<Point2<Ground>> = {
        let angle_generator = Uniform::from(-PI..PI);
        let radius_variance_generator = Uniform::from(-RADIUS_VARIANCE..RADIUS_VARIANCE);

        angle_generator
            .sample_iter(rng.clone())
            .take(POINTS_PER_CIRCLE)
            .zip(
                radius_variance_generator
                    .sample_iter(rng.clone())
                    .take(POINTS_PER_CIRCLE),
            )
            .map(|(angle, radius_change)| {
                let new_radius = radius + radius_change;
                let x = angle.cos() * new_radius + CIRCLE_CENTER_GROUND.x();
                let y = angle.sin() * new_radius + CIRCLE_CENTER_GROUND.y();
                point![x, y]
            })
            .collect_vec()
    };

    if rng.gen_range(0..10) > 8 {
        let projected_center = matrix.ground_to_pixel(CIRCLE_CENTER_GROUND)?;
        let final_circle: CenterOfCircleAndPoints<Pixel> = {
            let projected_points = circle_points
                .iter()
                .filter_map(|point| matrix.ground_to_pixel(*point).ok())
                .collect_vec();
            if projected_points.len() > 3 {
                Err(eyre!("expected at least 3 valid projected points"))
            } else {
                Ok(CenterOfCircleAndPoints::<Pixel> {
                    center: projected_center,
                    points: projected_points,
                })
            }
        }?;
        Ok(Measurement {
            matrix: matrix.clone(),
            position,
            circles: final_circle,
        })
    } else {
        Err(eyre!("don't have a measurement for you!"))
    }
}
