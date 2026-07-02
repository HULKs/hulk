use factrs::{core::Vector3, linalg::Matrix3};
use nalgebra::{Matrix2, SMatrix};
use types::field_dimensions::FieldDimensions;

pub struct BackendConfiguration {
    /// The spacing between control knots on the Gaussian Process
    /// Each control knot represents 9 DoFs for the optimizer.
    pub knot_spacing: std::time::Duration,
    /// The maximum optimization window size.
    /// Factors before the optimization window are marginalized.
    pub max_optimization_window: std::time::Duration,
    /// Maximum optimizer iterations per solve call.
    /// Slow solve cadences can spend more iterations on each larger batch.
    pub optimizer_max_iterations: usize,

    pub gyroscope_noise: Matrix3<f64>,
    pub accelerometer_noise: Matrix3<f64>,
    pub use_accelerometer_measurements: bool,
    pub gyroscope_process_noise: Matrix3<f64>,
    pub accelerometer_process_noise: Matrix3<f64>,
    pub gravity: Vector3<f64>,

    pub roll_pitch_yaw_noise: Matrix3<f64>,
    pub visual_feature_noise: Matrix2<f64>,
    pub pose_hint_visual_feature_noise: Matrix2<f64>,
    pub pose_hint_visual_huber_threshold: f64,
    pub visual_odometry_noise: SMatrix<f64, 6, 6>,
    pub foot_ground_sigma: f64,
    pub field_containment: FieldContainmentConfiguration,
}

#[derive(Debug, Clone, Copy)]
pub struct FieldContainmentConfiguration {
    pub x_limit: f64,
    pub y_limit: f64,
    pub sigma: f64,
}

impl FieldContainmentConfiguration {
    pub fn from_field_dimensions(field_dimensions: &FieldDimensions, sigma: f64) -> Self {
        Self {
            x_limit: field_dimensions.length as f64 * 0.5
                + field_dimensions.border_strip_width as f64,
            y_limit: field_dimensions.width as f64 * 0.5
                + field_dimensions.border_strip_width as f64,
            sigma,
        }
    }
}

impl Default for FieldContainmentConfiguration {
    fn default() -> Self {
        Self::from_field_dimensions(&FieldDimensions::SPL_2025, 1.0)
    }
}
