use std::time::Duration;

use context_attribute::context;
use framework::{
    AdditionalOutput, HistoricInput, MainOutput, Parameter, PerceptionInput, RequiredInput,
};
use nalgebra::{Isometry2, Vector2, Vector4};
use types::{
    Ball, BallFilterHypothesis, BallPosition, CameraMatrices, Circle, ProjectedLimbs, SensorData,
};

pub struct BallFilter {}

#[context]
pub struct NewContext {}

#[context]
pub struct CycleContext {
    pub hypothesis_timeout: Parameter<Duration, "control/ball_filter/hypothesis_timeout">,
    pub measurement_matching: Parameter<f32, "control/ball_filter/measurement_matching">,
    pub hypothesis_merge_distance: Parameter<f32, "control/ball_filter/hypothesis_merge_distance">,
    pub process_noise: Parameter<Vector4<f32>, "control/ball_filter/process_noise">,
    pub measurement_noise: Parameter<Vector2<f32>, "control/ball_filter/measurement_noise">,
    pub initial_covariance: Parameter<Vector4<f32>, "control/ball_filter/initial_covariance">,
    pub visible_validity_exponential_decay_factor:
        Parameter<f32, "control/ball_filter/visible_validity_exponential_decay_factor">,
    pub hidden_validity_exponential_decay_factor:
        Parameter<f32, "control/ball_filter/hidden_validity_exponential_decay_factor">,
    pub validity_discard_threshold:
        Parameter<f32, "control/ball_filter/validity_discard_threshold">,

    pub sensor_data: RequiredInput<SensorData, "sensor_data">,
    pub camera_matrices: RequiredInput<CameraMatrices, "camera_matrices">,

    pub historic_projected_limbs: HistoricInput<ProjectedLimbs, "projected_limbs">,
    pub historic_camera_matrices: HistoricInput<CameraMatrices, "camera_matrices">,
    pub historic_current_odometry_to_last_odometry:
        HistoricInput<Isometry2<f32>, "current_odometry_to_last_odometry">,

    pub balls_top: PerceptionInput<Option<Vec<Ball>>, "VisionTop", "balls">,
    pub balls_bottom: PerceptionInput<Option<Vec<Ball>>, "VisionBottom", "balls">,

    pub ball_filter_hypotheses:
        AdditionalOutput<Vec<BallFilterHypothesis>, "ball_filter_hypotheses">,
    pub filtered_balls_in_image_top: AdditionalOutput<Vec<Circle>, "filtered_balls_in_image_top">,
    pub filtered_balls_in_image_bottom:
        AdditionalOutput<Vec<Circle>, "filtered_balls_in_image_bottom">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub ball_position: MainOutput<Option<BallPosition>>,
}

impl BallFilter {
    pub fn new(context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
