use std::time::Duration;

use ros_z::Message;
use serde::{Deserialize, Serialize};

use crate::global_association::GlobalAssociationConfig as GlobalLocalizerParameters;

#[derive(Clone, Debug, Deserialize, Serialize, Message)]
#[serde(default, deny_unknown_fields)]
pub struct FieldMarkAssociationParameters {
    /// Maximum separation of exposure-bracketing attitude samples.
    pub max_imu_gap: Duration,
    /// Maximum separation of exposure-bracketing camera geometry samples.
    pub max_camera_gap: Duration,
    /// Allocated at startup; changing these requires restarting the node.
    pub capacities: AssociationCapacities,
    /// Shared projection uncertainty/search budgets and global geometric gates.
    pub global_localizer: GlobalLocalizerParameters,
    pub tracking: TrackingAssociationParameters,
    /// Maximum node-side timestamp difference for association geometry.
    ///
    /// The direct API expects geometry already sampled at the detection time.
    pub max_pose_hint_age: Duration,
}

impl Default for FieldMarkAssociationParameters {
    fn default() -> Self {
        Self {
            max_imu_gap: Duration::from_millis(20),
            max_camera_gap: Duration::from_millis(20),
            capacities: AssociationCapacities::default(),
            global_localizer: GlobalLocalizerParameters::default(),
            tracking: TrackingAssociationParameters::default(),
            max_pose_hint_age: Duration::from_millis(250),
        }
    }
}

impl FieldMarkAssociationParameters {
    pub(crate) fn validate(&self) -> std::result::Result<(), String> {
        self.global_localizer.validate()?;
        self.tracking.validate()?;
        self.capacities.validate()?;
        if self.max_pose_hint_age.is_zero()
            || self.max_imu_gap.is_zero()
            || self.max_camera_gap.is_zero()
        {
            return Err("association timing limits must be > 0".to_string());
        }
        Ok(())
    }

    pub(crate) fn validate_update(&self, capacities: AssociationCapacities) -> Result<(), String> {
        self.validate()?;
        if self.capacities != capacities {
            return Err("association capacities changed: restart required".into());
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize, Message)]
#[serde(default, deny_unknown_fields)]
pub struct AssociationCapacities {
    pub camera_geometry: usize,
    pub field_dimensions: usize,
    pub estimates: usize,
    pub status: usize,
    pub attitudes: usize,
    pub imu_queue: usize,
    pub detections_queue: usize,
}

impl Default for AssociationCapacities {
    fn default() -> Self {
        Self {
            camera_geometry: 1500,
            field_dimensions: 1,
            estimates: 128,
            status: 1,
            attitudes: 1500,
            imu_queue: 500,
            detections_queue: 1,
        }
    }
}

impl AssociationCapacities {
    fn validate(self) -> Result<(), String> {
        if [
            self.camera_geometry,
            self.field_dimensions,
            self.estimates,
            self.status,
            self.attitudes,
            self.imu_queue,
            self.detections_queue,
        ]
        .contains(&0)
        {
            return Err("association capacities must be > 0".into());
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Message)]
#[serde(default, deny_unknown_fields)]
pub struct TrackingAssociationParameters {
    /// Maximum absolute entry of covariance minus its transpose.
    pub covariance_symmetry_tolerance: f32,
    /// Allowed negative eigenvalue magnitude from covariance roundoff.
    pub covariance_psd_tolerance: f32,
    /// Bounded covariance eigensolver iterations; zero (unbounded) is rejected.
    pub covariance_eigen_max_iterations: usize,
    /// Pixel residual ceiling for winning edges. Plausible rivals beyond it still enter scoring.
    pub max_pixel_distance: f32,
    /// Additional position sigma per second without a successful solve.
    pub position_sigma_per_second: f32,
    /// Additional angular sigma per second without a successful solve.
    pub yaw_sigma_per_second: f32,
    /// Validity horizon since the last successful solve; older predictions reject without fallback.
    pub max_age: Duration,
    /// For 3..=5 features, required best/rival joint Gaussian likelihood ratio;
    /// the log-likelihood gap must exceed ln(score_ratio).
    /// For >5 features, marginal-assignment heuristic: removing any winning edge must lose
    /// more than 1 - 1/score_ratio of that edge's normalized Gaussian benefit.
    pub score_ratio: f32,
}

impl Default for TrackingAssociationParameters {
    fn default() -> Self {
        Self {
            covariance_symmetry_tolerance: 1.0e-5,
            covariance_psd_tolerance: 1.0e-6,
            covariance_eigen_max_iterations: 64,
            max_pixel_distance: 80.0,
            position_sigma_per_second: 0.15,
            yaw_sigma_per_second: 0.1,
            max_age: Duration::from_secs(5),
            score_ratio: 1.05,
        }
    }
}

impl TrackingAssociationParameters {
    fn validate(&self) -> Result<(), String> {
        for value in [
            self.covariance_symmetry_tolerance,
            self.covariance_psd_tolerance,
            self.max_pixel_distance,
            self.position_sigma_per_second,
            self.yaw_sigma_per_second,
        ] {
            if !value.is_finite() || value <= 0.0 {
                return Err(
                    "tracking distance, uncertainty rates and tolerances must be finite and > 0"
                        .into(),
                );
            }
        }
        if self.covariance_eigen_max_iterations == 0
            || self.max_age.is_zero()
            || !self.score_ratio.is_finite()
            || self.score_ratio <= 1.0
        {
            return Err("invalid tracking eigensolver iteration limit, age or score ratio".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ros_z::{context::ContextBuilder, parameter::NodeParametersExt};
    use std::sync::Arc;

    #[test]
    fn defaults_and_detection_limits_remain_compatible() {
        let parameters: FieldMarkAssociationParameters = serde_json::from_str("{}").unwrap();
        parameters.validate().unwrap();
        assert_eq!(parameters.global_localizer.max_retained_detections, 32);
        let mut config = parameters.global_localizer;
        config.max_retained_detections = 64;
        config.max_work = 2_000_000;
        config.validate().unwrap();
        config.min_inliers = 2;
        assert!(config.validate().is_err());
        config.min_inliers = 3;
        config.max_input_detections = 63;
        assert!(config.validate().is_err());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn capacity_updates_preserve_published_snapshot_and_live_settings_reload() {
        let root =
            std::env::temp_dir().join(format!("association-parameters-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let file = root.join("field_mark_association.json5");
        let mut config = FieldMarkAssociationParameters::default();
        config.capacities.imu_queue = 17;
        std::fs::write(&file, serde_json::to_string(&config).unwrap()).unwrap();
        let context = ContextBuilder::default()
            .with_mode("peer")
            .disable_multicast_scouting()
            .with_parameter_layers([root.clone()])
            .build()
            .await
            .unwrap();
        let node = context
            .create_node("association_parameters_test")
            .build()
            .await
            .unwrap();
        let parameters = node
            .bind_parameter_as::<FieldMarkAssociationParameters>("field_mark_association")
            .unwrap();
        let capacities = parameters.snapshot().typed.capacities;
        assert_eq!(capacities.imu_queue, 17);
        parameters
            .add_validation_hook(move |candidate| candidate.validate_update(capacities))
            .unwrap();
        let original = parameters.snapshot();
        let original_file = std::fs::read(&file).unwrap();
        assert!(
            parameters
                .set_json(
                    "capacities.imu_queue",
                    serde_json::json!(18),
                    root.to_string_lossy().into_owned(),
                )
                .unwrap_err()
                .to_string()
                .contains("restart required")
        );
        assert_eq!(std::fs::read(&file).unwrap(), original_file);
        assert!(Arc::ptr_eq(&original, &parameters.snapshot()));
        for change in 0..7 {
            config.capacities = capacities;
            let capacity = match change {
                0 => &mut config.capacities.camera_geometry,
                1 => &mut config.capacities.field_dimensions,
                2 => &mut config.capacities.estimates,
                3 => &mut config.capacities.status,
                4 => &mut config.capacities.attitudes,
                5 => &mut config.capacities.imu_queue,
                _ => &mut config.capacities.detections_queue,
            };
            *capacity += 1;
            std::fs::write(&file, serde_json::to_string(&config).unwrap()).unwrap();
            assert!(
                parameters
                    .reload()
                    .unwrap_err()
                    .to_string()
                    .contains("restart required")
            );
            assert!(Arc::ptr_eq(&original, &parameters.snapshot()));
        }
        config.capacities = capacities;
        config.max_imu_gap = Duration::from_millis(37);
        config.global_localizer.symmetry_epsilon = 0.002;
        std::fs::write(&file, serde_json::to_string(&config).unwrap()).unwrap();
        parameters.reload().unwrap();
        let updated = parameters.snapshot();
        assert_eq!(updated.typed.max_imu_gap, config.max_imu_gap);
        assert_eq!(updated.typed.global_localizer.symmetry_epsilon, 0.002);
        assert!(!Arc::ptr_eq(&original.typed, &updated.typed));
        config.capacities.imu_queue = 0;
        assert!(config.validate().is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}
