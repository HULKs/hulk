use std::time::Duration;

use coordinate_systems::Ground;
use linear_algebra::Point2;
use nalgebra::{Matrix2, Matrix4, Vector2, Vector4};

#[derive(Clone, Debug)]
pub struct TrackerParameters {
    pub rolling_velocity_decay: f32,
    pub mode_transition_static_to_rolling: f32,
    pub mode_transition_rolling_to_static: f32,
    pub max_ball_speed: f32,
    pub gating_chi_square: f32,
    pub probability_of_detection_visible: f32,
    pub probability_of_detection_hidden: f32,
    pub survival_probability: f32,
    pub clutter_log_likelihood: f32,
    pub birth_log_likelihood: f32,
    pub max_assignments_per_hypothesis: usize,
    pub birth_existence_probability: f32,
    pub confirm_existence_threshold: f32,
    pub delete_existence_threshold: f32,
    pub minimum_confirming_hits: u32,
    pub tentative_timeout: Duration,
    pub confirmed_timeout: Duration,
    pub stale_timeout: Duration,
    pub max_global_hypotheses: usize,
    pub max_tracks_per_hypothesis: usize,
    pub hypothesis_prune_log_weight: f32,
    pub merge_distance: f32,
    pub static_process_noise: Matrix2<f32>,
    pub rolling_process_noise: Matrix4<f32>,
    pub initial_position_covariance: Vector2<f32>,
    pub initial_velocity_covariance: Vector2<f32>,
}

impl Default for TrackerParameters {
    fn default() -> Self {
        Self {
            rolling_velocity_decay: 0.998,
            mode_transition_static_to_rolling: 0.03,
            mode_transition_rolling_to_static: 0.08,
            max_ball_speed: 6.0,
            gating_chi_square: 5.991,
            probability_of_detection_visible: 0.85,
            probability_of_detection_hidden: 0.05,
            survival_probability: 0.995,
            clutter_log_likelihood: -5.0,
            birth_log_likelihood: -5.0,
            max_assignments_per_hypothesis: 8,
            birth_existence_probability: 0.35,
            confirm_existence_threshold: 0.7,
            delete_existence_threshold: 0.1,
            minimum_confirming_hits: 2,
            tentative_timeout: Duration::from_secs(1),
            confirmed_timeout: Duration::from_secs(5),
            stale_timeout: Duration::from_secs(2),
            max_global_hypotheses: 16,
            max_tracks_per_hypothesis: 15,
            hypothesis_prune_log_weight: -8.0,
            merge_distance: 0.1,
            static_process_noise: Matrix2::from_diagonal(&Vector2::new(0.001, 0.001)),
            rolling_process_noise: Matrix4::from_diagonal(&Vector4::new(0.005, 0.005, 0.02, 0.02)),
            initial_position_covariance: Vector2::new(0.5, 0.5),
            initial_velocity_covariance: Vector2::new(40.0, 40.0),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct FieldBounds {
    pub x_limit: f32,
    pub y_limit: f32,
}

impl FieldBounds {
    pub fn contains(&self, position: Point2<Ground>) -> bool {
        position.x().abs() <= self.x_limit && position.y().abs() <= self.y_limit
    }
}
