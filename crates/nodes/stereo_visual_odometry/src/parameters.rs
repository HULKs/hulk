use std::path::PathBuf;

use ros_z::Message;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, Message)]
#[serde(deny_unknown_fields)]
pub struct StereoVisualOdometryParameters {
    pub enable: bool,
    pub neural_network: PathBuf,
    pub pose_estimation_parameters: StereoVisualOdometryPoseEstimationParameters,
}

#[derive(Clone, Debug, Deserialize, Serialize, Message)]
#[serde(deny_unknown_fields)]
pub struct StereoVisualOdometryPoseEstimationParameters {
    pub minimum_pnp_correspondences: usize,
    pub ransac_reprojection_threshold_px: f32,
    pub ransac_max_iterations: usize,
    pub ransac_confidence: f32,
    pub lm_max_iterations: usize,
    pub lm_initial_lambda: f32,
    pub lm_min_lambda: f32,
    pub lm_max_lambda: f32,
    pub lm_step_tolerance: f32,
    pub lm_cost_tolerance: f32,
    pub lm_huber_threshold_px: f32,
    pub full_weight_disparity_px: f32,
    pub min_disparity_weight: f32,
    pub max_vertical_disparity_px: f32,
}

impl StereoVisualOdometryParameters {
    pub fn validate(&self) -> Result<(), String> {
        if !self.neural_network.is_file() {
            return Err("neural_network must be a valid file path".to_string());
        }

        self.pose_estimation_parameters.validate()
    }
}

impl StereoVisualOdometryPoseEstimationParameters {
    pub fn validate(&self) -> Result<(), String> {
        if self.minimum_pnp_correspondences < 4 {
            return Err("minimum_pnp_correspondences must be at least 4".to_string());
        }

        if !self.ransac_confidence.is_finite()
            || self.ransac_confidence <= 0.0
            || self.ransac_confidence >= 1.0
        {
            return Err("ransac_confidence must be finite and in (0, 1)".to_string());
        }

        if !self.ransac_reprojection_threshold_px.is_finite()
            || self.ransac_reprojection_threshold_px <= 0.0
        {
            return Err("ransac_reprojection_threshold_px must be finite and > 0".to_string());
        }

        if self.ransac_max_iterations < 1 {
            return Err("ransac_max_iterations must be > 0".to_string());
        }

        if self.lm_max_iterations < 1 {
            return Err("lm_max_iterations must be > 0".to_string());
        }

        if !self.lm_min_lambda.is_finite() || self.lm_min_lambda <= 0.0 {
            return Err("lm_min_lambda must be > 0".to_string());
        }

        if !self.lm_initial_lambda.is_finite() || self.lm_initial_lambda < self.lm_min_lambda {
            return Err("lm_initial_lambda must be >= lm_min_lambda".to_string());
        }

        if !self.lm_max_lambda.is_finite() || self.lm_max_lambda <= 0.0 {
            return Err("lm_max_lambda must be > 0".to_string());
        }

        if self.lm_max_lambda < self.lm_initial_lambda
            || self.lm_initial_lambda < self.lm_min_lambda
        {
            return Err(
                "LM lambda values must satisfy lm_min_lambda <= lm_initial_lambda <= lm_max_lambda"
                    .to_string(),
            );
        }

        if !self.lm_step_tolerance.is_finite() || self.lm_step_tolerance <= 0.0 {
            return Err("lm_step_tolerance must be finite and > 0".to_string());
        }

        if !self.lm_cost_tolerance.is_finite() || self.lm_cost_tolerance <= 0.0 {
            return Err("lm_cost_tolerance must be finite and > 0".to_string());
        }

        if !self.lm_huber_threshold_px.is_finite() || self.lm_huber_threshold_px <= 0.0 {
            return Err("lm_huber_threshold_px must be finite and > 0".to_string());
        }

        if !self.full_weight_disparity_px.is_finite() || self.full_weight_disparity_px <= 0.0 {
            return Err("full_weight_disparity_px must be finite and > 0".to_string());
        }

        if !self.min_disparity_weight.is_finite()
            || self.min_disparity_weight <= 0.0
            || self.min_disparity_weight > 1.0
        {
            return Err("min_disparity_weight must be finite and in (0, 1]".to_string());
        }

        if !self.max_vertical_disparity_px.is_finite() || self.max_vertical_disparity_px < 0.0 {
            return Err("max_vertical_disparity_px must be finite and >= 0".to_string());
        }

        Ok(())
    }
}
