use std::time::Duration;

use localization_factrs::{BackendConfiguration, FieldContainmentConfiguration};
use nalgebra::{Matrix2, Matrix3, SMatrix, Vector3, vector};
use ros_z::Message;
use serde::{Deserialize, Serialize};
use types::field_dimensions::FieldDimensions;

/// Runtime parameters for the 3D localization node.
#[derive(Clone, Debug, Deserialize, Serialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Localization3dParameters {
    /// Pixel residual variance for accepted visual feature associations.
    pub visual_feature_noise_variance: f64,
    /// Pixel residual variance for lower-trust pose-hint visual feature associations.
    pub pose_hint_visual_feature_noise_variance: f64,
    /// Huber threshold for pose-hint visual residuals in whitened residual units.
    pub pose_hint_visual_huber_threshold: f64,
    /// Soft field containment sigma in meters outside field plus border strip.
    pub field_containment_sigma: f64,
}

impl Localization3dParameters {
    pub(crate) fn validate(&self) -> std::result::Result<(), String> {
        if !self.visual_feature_noise_variance.is_finite()
            || self.visual_feature_noise_variance <= 0.0
        {
            return Err("visual_feature_noise_variance must be finite and > 0".to_string());
        }
        if !self.pose_hint_visual_feature_noise_variance.is_finite()
            || self.pose_hint_visual_feature_noise_variance <= 0.0
        {
            return Err(
                "pose_hint_visual_feature_noise_variance must be finite and > 0".to_string(),
            );
        }
        if !self.pose_hint_visual_huber_threshold.is_finite()
            || self.pose_hint_visual_huber_threshold <= 0.0
        {
            return Err("pose_hint_visual_huber_threshold must be finite and > 0".to_string());
        }
        if !self.field_containment_sigma.is_finite() || self.field_containment_sigma <= 0.0 {
            return Err("field_containment_sigma must be finite and > 0".to_string());
        }
        Ok(())
    }
}

pub(crate) fn backend_configuration_from_parameters_and_field_dimensions(
    parameters: &Localization3dParameters,
    field_dimensions: &FieldDimensions,
) -> BackendConfiguration {
    backend_configuration_with_pose_hint(
        parameters.visual_feature_noise_variance,
        parameters.pose_hint_visual_feature_noise_variance,
        parameters.pose_hint_visual_huber_threshold,
        parameters.field_containment_sigma,
        field_dimensions,
    )
}

fn backend_configuration_with_pose_hint(
    visual_feature_noise_variance: f64,
    pose_hint_visual_feature_noise_variance: f64,
    pose_hint_visual_huber_threshold: f64,
    field_containment_sigma: f64,
    field_dimensions: &FieldDimensions,
) -> BackendConfiguration {
    let process_noise = Matrix3::identity() * 0.01;
    BackendConfiguration {
        knot_spacing: Duration::from_millis(200),
        max_optimization_window: Duration::from_secs(2),
        optimizer_max_iterations: 2,
        gyroscope_noise: Matrix3::identity() * 0.1_f64.powi(2),
        // TODO: tune accelerometer noise
        accelerometer_noise: Matrix3::from_diagonal(&vector![
            5.0_f64.powi(2),   // x sigma = 5 m/s^2
            5.0_f64.powi(2),   // y sigma = 5 m/s^2
            100.0_f64.powi(2), // z disabled
        ]),
        use_accelerometer_measurements: false,
        gyroscope_process_noise: process_noise,
        roll_pitch_yaw_noise: Matrix3::from_diagonal(&Vector3::new(0.01, 0.01, 0.00001)),
        accelerometer_process_noise: process_noise,
        visual_feature_noise: Matrix2::identity() * visual_feature_noise_variance,
        pose_hint_visual_feature_noise: Matrix2::identity()
            * pose_hint_visual_feature_noise_variance,
        pose_hint_visual_huber_threshold,
        // factrs::SE3 tangent order is [rot_x, rot_y, rot_z, trans_x, trans_y, trans_z].
        visual_odometry_noise: SMatrix::<f64, 6, 6>::identity() * 1.0e-2,
        foot_ground_sigma: 1e-2,
        field_containment: FieldContainmentConfiguration::from_field_dimensions(
            field_dimensions,
            field_containment_sigma,
        ),
        gravity: Vector3::new(0.0, 0.0, 9.81),
    }
}
