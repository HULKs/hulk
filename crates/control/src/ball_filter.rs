use std::time::Duration;

use context_attribute::context;
use framework::{
    AdditionalOutput, HistoricInput, MainOutput, PerceptionInput,
};
use nalgebra::{Isometry2, Vector2, Vector4};
use types::{
    Ball, BallFilterHypothesis, BallPosition, CameraMatrices, Circle, FieldDimensions,
    ProjectedLimbs, SensorData,
};

pub struct BallFilter {}

#[context]
pub struct NewContext {
    pub field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    pub hidden_validity_exponential_decay_factor:
        Parameter<f32, "control/ball_filter/hidden_validity_exponential_decay_factor">,
    pub hypothesis_merge_distance: Parameter<f32, "control/ball_filter/hypothesis_merge_distance">,
    pub hypothesis_timeout: Parameter<Duration, "control/ball_filter/hypothesis_timeout">,
    pub initial_covariance: Parameter<Vector4<f32>, "control/ball_filter/initial_covariance">,
    pub measurement_matching_distance:
        Parameter<f32, "control/ball_filter/measurement_matching_distance">,
    pub measurement_noise: Parameter<Vector2<f32>, "control/ball_filter/measurement_noise">,
    pub process_noise: Parameter<Vector4<f32>, "control/ball_filter/process_noise">,
    pub validity_discard_threshold:
        Parameter<f32, "control/ball_filter/validity_discard_threshold">,
    pub visible_validity_exponential_decay_factor:
        Parameter<f32, "control/ball_filter/visible_validity_exponential_decay_factor">,
}

#[context]
pub struct CycleContext {
    pub ball_filter_hypotheses:
        AdditionalOutput<Vec<BallFilterHypothesis>, "ball_filter_hypotheses">,
    pub filtered_balls_in_image_bottom:
        AdditionalOutput<Vec<Circle>, "filtered_balls_in_image_bottom">,
    pub filtered_balls_in_image_top: AdditionalOutput<Vec<Circle>, "filtered_balls_in_image_top">,

    pub current_odometry_to_last_odometry:
        HistoricInput<Option<Isometry2<f32>>, "current_odometry_to_last_odometry?">,
    pub historic_camera_matrices: HistoricInput<Option<CameraMatrices>, "camera_matrices?">,
    pub projected_limbs: HistoricInput<Option<ProjectedLimbs>, "projected_limbs?">,

    pub camera_matrices: RequiredInput<Option<CameraMatrices>, "camera_matrices?">,
    pub sensor_data: Input<SensorData, "sensor_data">,

    pub field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    pub hidden_validity_exponential_decay_factor:
        Parameter<f32, "control/ball_filter/hidden_validity_exponential_decay_factor">,
    pub hypothesis_merge_distance: Parameter<f32, "control/ball_filter/hypothesis_merge_distance">,
    pub hypothesis_timeout: Parameter<Duration, "control/ball_filter/hypothesis_timeout">,
    pub initial_covariance: Parameter<Vector4<f32>, "control/ball_filter/initial_covariance">,
    pub measurement_matching_distance:
        Parameter<f32, "control/ball_filter/measurement_matching_distance">,
    pub measurement_noise: Parameter<Vector2<f32>, "control/ball_filter/measurement_noise">,
    pub process_noise: Parameter<Vector4<f32>, "control/ball_filter/process_noise">,
    pub validity_discard_threshold:
        Parameter<f32, "control/ball_filter/validity_discard_threshold">,
    pub visible_validity_exponential_decay_factor:
        Parameter<f32, "control/ball_filter/visible_validity_exponential_decay_factor">,

    pub balls_bottom: PerceptionInput<Option<Vec<Ball>>, "VisionBottom", "balls?">,
    pub balls_top: PerceptionInput<Option<Vec<Ball>>, "VisionTop", "balls?">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub ball_position: MainOutput<Option<BallPosition>>,
}

impl BallFilter {
    pub fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, _context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
