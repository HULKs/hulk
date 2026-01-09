use color_eyre::Result;
use nalgebra::Matrix2;
use serde::{Deserialize, Serialize};

use context_attribute::context;
use framework::{HistoricInput, MainOutput, PerceptionInput};
use geometry::circle::Circle;
use linear_algebra::IntoFramed;
use projection::{camera_matrix::CameraMatrix, Projection};
use types::{
    ball_detection::BallPercept, multivariate_normal_distribution::MultivariateNormalDistribution,
    object_detection::Detection, parameters::BallDetectionParameters,
};

#[derive(Deserialize, Serialize)]
pub struct BallProjector {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    past_camera_matrices: HistoricInput<Option<CameraMatrix>, "camera_matrix?">,
    detected_objects: PerceptionInput<Vec<Detection>, "ObjectDetection", "detected_objects">,

    parameters: Parameter<BallDetectionParameters, "ball_detection">,
    ball_radius: Parameter<f32, "field_dimensions.ball_radius">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub balls: MainOutput<Option<Vec<BallPercept>>>,
}

impl BallProjector {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let balls: Vec<BallPercept> = context
            .detected_objects
            .persistent
            .iter()
            .flat_map(|(time, detections)| {
                detections
                    .iter()
                    .copied()
                    .flatten()
                    .flat_map(|detection| {
                        if detection.label != "sports ball" {
                            return None;
                        }

                        let area = detection.bounding_box.area;
                        let camera_matrix = context
                            .past_camera_matrices
                            .get(time)
                            .expect("no camera matrix found for detected object");

                        let position = camera_matrix
                            .pixel_to_ground_with_z(area.center(), *context.ball_radius)
                            .ok()?;

                        let detected_ball_radius =
                            (area.max.x() - area.min.x()).min(area.max.y() - area.min.y());

                        let circle = Circle {
                            center: area.center(),
                            radius: detected_ball_radius,
                        };

                        let projected_covariance = {
                            let distance = position.coords().norm();
                            let distance_noise_increase = 1.0
                                + (distance - context.parameters.noise_increase_distance_threshold)
                                    .max(0.0)
                                    * context.parameters.noise_increase_slope;

                            let scaled_noise = context
                                .parameters
                                .detection_noise
                                .inner
                                .map(|x| (detected_ball_radius * x).powi(2))
                                .framed();
                            camera_matrix
                                .project_noise_to_ground(position, scaled_noise)
                                .ok()?
                                * (Matrix2::identity() * distance_noise_increase.powi(2))
                        };

                        Some(BallPercept {
                            percept_in_ground: MultivariateNormalDistribution {
                                mean: position.inner.coords,
                                covariance: projected_covariance,
                            },
                            image_location: circle,
                        })
                    })
                    .collect::<Vec<BallPercept>>()
            })
            .collect();

        Ok(MainOutputs {
            balls: Some(balls).into(),
        })
    }
}
