use std::time::Duration;

use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

use field_mark_association::FieldMarkAssociationParameters;
use localization_3d::Localization3dParameters;

/// Fixed logical simulation tick.
pub const TICK_INTERVAL: Duration = Duration::from_millis(20);
/// Fixed backend solve cadence.
/// Fixed synthetic field-mark cadence.
pub const FIELD_MARK_INTERVAL: Duration = Duration::from_millis(100);

/// Loads the bundled production 3D-localization parameters.
pub fn production_localization_parameters() -> Result<Localization3dParameters, String> {
    json5::from_str(include_str!(
        "../../../etc/parameters/base/localization3d.json5"
    ))
    .map_err(|error| format!("failed to parse production localization3d parameters: {error}"))
}

/// Loads the bundled production field-mark association parameters.
pub fn production_association_parameters() -> Result<FieldMarkAssociationParameters, String> {
    json5::from_str(include_str!(
        "../../../etc/parameters/base/field_mark_association.json5"
    ))
    .map_err(|error| {
        format!("failed to parse production field-mark association parameters: {error}")
    })
}

/// Selects whether field points bypass or exercise production association.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum AssociationMode {
    /// Explicit isolation mode: bypass association with known ground-truth correspondences.
    KnownCorrespondences,
    /// Feed class-grouped pixels through the production stateless associator.
    #[default]
    ProductionAssociation,
}

/// Selects the visual-odometry input path exercised by the simulation.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum VisualOdometryMode {
    /// Use deterministic synthetic camera-to-camera deltas.
    #[default]
    SyntheticDelta,
    /// Exercise the production stereo visual-odometry path.
    ProductionStereo,
}

/// A deterministic perturbation applied once to the indexed VO transition.
/// Transition zero is the transition from the first frame to the second frame.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VisualOdometryOutlier {
    /// Zero-based visual-odometry transition on which to inject the fault.
    pub transition_index: usize,
    /// Translation perturbation in meters.
    pub translation: [f32; 3],
    /// Rotation perturbation represented as a scaled axis in radians.
    pub rotation_scaled_axis: [f32; 3],
}

/// Deterministic synthetic sensor settings used to construct a simulation.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SimulationConfig {
    /// Seed used to derive independent deterministic sensor random streams.
    pub seed: u64,
    /// Field-mark association path exercised by the simulation.
    pub association_mode: AssociationMode,
    /// Visual-odometry path exercised by the simulation.
    pub visual_odometry_mode: VisualOdometryMode,
    /// Delay of the right camera relative to the left camera in milliseconds.
    pub right_camera_delay_ms: f32,
    /// Whether production stereo should treat left and right timestamps as synchronized.
    pub assume_synchronized_stereo_timestamps: bool,
    /// Standard deviation of pixel noise.
    pub landmark_pixel_sigma: f32,
    /// Independent probability of dropping each visible landmark.
    pub landmark_dropout_probability: f32,
    /// Standard deviation of visual-odometry translation noise in meters.
    pub vo_translation_sigma_m: f32,
    /// Standard deviation of visual-odometry rotation noise in radians.
    pub vo_rotation_sigma_rad: f32,
    /// Translation bias applied to every visual-odometry transition.
    pub vo_translation_bias_per_step: [f32; 3],
    /// Scaled-axis rotation bias applied to every visual-odometry transition.
    pub vo_rotation_bias_per_step: [f32; 3],
    /// Optional deterministic one-shot visual-odometry fault.
    pub vo_outlier: Option<VisualOdometryOutlier>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SimulationConfigDto {
    seed: u64,
    association_mode: AssociationMode,
    #[serde(default)]
    visual_odometry_mode: VisualOdometryMode,
    #[serde(default)]
    right_camera_delay_ms: f32,
    #[serde(default = "default_true")]
    assume_synchronized_stereo_timestamps: bool,
    landmark_pixel_sigma: f32,
    landmark_dropout_probability: f32,
    vo_translation_sigma_m: f32,
    vo_rotation_sigma_rad: f32,
    vo_translation_bias_per_step: [f32; 3],
    vo_rotation_bias_per_step: [f32; 3],
    vo_outlier: Option<VisualOdometryOutlier>,
}

impl<'de> Deserialize<'de> for SimulationConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let dto = SimulationConfigDto::deserialize(deserializer)?;
        let config = Self {
            seed: dto.seed,
            association_mode: dto.association_mode,
            visual_odometry_mode: dto.visual_odometry_mode,
            right_camera_delay_ms: dto.right_camera_delay_ms,
            assume_synchronized_stereo_timestamps: dto.assume_synchronized_stereo_timestamps,
            landmark_pixel_sigma: dto.landmark_pixel_sigma,
            landmark_dropout_probability: dto.landmark_dropout_probability,
            vo_translation_sigma_m: dto.vo_translation_sigma_m,
            vo_rotation_sigma_rad: dto.vo_rotation_sigma_rad,
            vo_translation_bias_per_step: dto.vo_translation_bias_per_step,
            vo_rotation_bias_per_step: dto.vo_rotation_bias_per_step,
            vo_outlier: dto.vo_outlier,
        };
        config.validate().map_err(D::Error::custom)?;
        Ok(config)
    }
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            seed: 0,
            association_mode: AssociationMode::ProductionAssociation,
            visual_odometry_mode: VisualOdometryMode::SyntheticDelta,
            right_camera_delay_ms: 0.0,
            assume_synchronized_stereo_timestamps: true,
            landmark_pixel_sigma: 0.5,
            landmark_dropout_probability: 0.0,
            vo_translation_sigma_m: 0.001,
            vo_rotation_sigma_rad: 0.001,
            vo_translation_bias_per_step: [0.0; 3],
            vo_rotation_bias_per_step: [0.0; 3],
            vo_outlier: None,
        }
    }
}

impl SimulationConfig {
    /// Returns the default deterministic large VO fault used by the built-in fault scenario.
    pub fn diagnostic_vo_fault() -> VisualOdometryOutlier {
        VisualOdometryOutlier {
            transition_index: 50,
            translation: [5.0, 0.0, 0.0],
            rotation_scaled_axis: [0.0, 0.0, 170.0_f32.to_radians()],
        }
    }

    /// Validates finite noise, bias, dropout, and outlier values.
    pub fn validate(&self) -> Result<(), String> {
        if !self.right_camera_delay_ms.is_finite()
            || !(0.0..=20.0).contains(&self.right_camera_delay_ms)
        {
            return Err("right_camera_delay_ms must be finite and in [0, 20]".to_string());
        }
        validate_non_negative(self.landmark_pixel_sigma, "landmark_pixel_sigma")?;
        if !self.landmark_dropout_probability.is_finite()
            || !(0.0..=1.0).contains(&self.landmark_dropout_probability)
        {
            return Err("landmark_dropout_probability must be finite and in [0, 1]".to_string());
        }
        validate_non_negative(self.vo_translation_sigma_m, "vo_translation_sigma_m")?;
        validate_non_negative(self.vo_rotation_sigma_rad, "vo_rotation_sigma_rad")?;
        validate_vector(
            self.vo_translation_bias_per_step,
            "vo_translation_bias_per_step",
        )?;
        validate_vector(self.vo_rotation_bias_per_step, "vo_rotation_bias_per_step")?;
        if let Some(outlier) = &self.vo_outlier {
            validate_vector(outlier.translation, "vo_outlier.translation")?;
            validate_vector(
                outlier.rotation_scaled_axis,
                "vo_outlier.rotation_scaled_axis",
            )?;
        }
        if self.visual_odometry_mode == VisualOdometryMode::ProductionStereo
            && (self.vo_translation_sigma_m != 0.0
                || self.vo_rotation_sigma_rad != 0.0
                || self.vo_translation_bias_per_step != [0.0; 3]
                || self.vo_rotation_bias_per_step != [0.0; 3]
                || self.vo_outlier.is_some())
        {
            return Err(
                "ProductionStereo requires zero synthetic VO noise/bias and vo_outlier: null"
                    .to_string(),
            );
        }
        Ok(())
    }
}

fn default_true() -> bool {
    true
}

fn validate_non_negative(value: f32, name: &str) -> Result<(), String> {
    if value.is_finite() && value >= 0.0 {
        Ok(())
    } else {
        Err(format!("{name} must be finite and >= 0"))
    }
}

fn validate_vector(value: [f32; 3], name: &str) -> Result<(), String> {
    if value.into_iter().all(f32::is_finite) {
        Ok(())
    } else {
        Err(format!("{name} must contain only finite values"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn production_stereo_rejects_synthetic_perturbations() {
        let config = SimulationConfig {
            visual_odometry_mode: VisualOdometryMode::ProductionStereo,
            vo_translation_sigma_m: 0.0,
            vo_rotation_sigma_rad: 0.0,
            ..Default::default()
        };
        assert!(config.validate().is_ok());
        for incompatible in [
            SimulationConfig {
                vo_translation_sigma_m: 0.001,
                ..config.clone()
            },
            SimulationConfig {
                vo_rotation_sigma_rad: 0.001,
                ..config.clone()
            },
            SimulationConfig {
                vo_translation_bias_per_step: [0.001; 3],
                ..config.clone()
            },
            SimulationConfig {
                vo_rotation_bias_per_step: [0.001; 3],
                ..config.clone()
            },
            SimulationConfig {
                vo_outlier: Some(SimulationConfig::diagnostic_vo_fault()),
                ..config
            },
        ] {
            assert!(incompatible.validate().is_err());
        }
    }

    #[test]
    fn unknown_configuration_fields_are_rejected() {
        let input = r#"{
            seed: 0,
            association_mode: "KnownCorrespondences",
            landmark_pixel_sigma: 0.0,
            landmark_dropout_probability: 0.0,
            vo_translation_sigma_m: 0.0,
            vo_rotation_sigma_rad: 0.0,
            vo_translation_bias_per_step: [0.0, 0.0, 0.0],
            vo_rotation_bias_per_step: [0.0, 0.0, 0.0],
            vo_outlier: null,
            misspelled_setting: 1,
        }"#;

        assert!(json5::from_str::<SimulationConfig>(input).is_err());
    }

    #[test]
    fn legacy_configuration_uses_visual_odometry_defaults() {
        let input = r#"{
            seed: 0,
            association_mode: "KnownCorrespondences",
            landmark_pixel_sigma: 0.0,
            landmark_dropout_probability: 0.0,
            vo_translation_sigma_m: 0.0,
            vo_rotation_sigma_rad: 0.0,
            vo_translation_bias_per_step: [0.0, 0.0, 0.0],
            vo_rotation_bias_per_step: [0.0, 0.0, 0.0],
            vo_outlier: null,
        }"#;

        let config = json5::from_str::<SimulationConfig>(input).unwrap();
        assert_eq!(
            config.visual_odometry_mode,
            VisualOdometryMode::SyntheticDelta
        );
        assert_eq!(config.right_camera_delay_ms, 0.0);
        assert!(config.assume_synchronized_stereo_timestamps);
    }

    #[test]
    fn right_camera_delay_must_be_finite_and_in_range() {
        for delay in [f32::NAN, f32::INFINITY, -0.1, 20.1] {
            assert!(
                SimulationConfig {
                    right_camera_delay_ms: delay,
                    ..Default::default()
                }
                .validate()
                .is_err()
            );
        }
        for delay in [0.0, 20.0] {
            assert!(
                SimulationConfig {
                    right_camera_delay_ms: delay,
                    ..Default::default()
                }
                .validate()
                .is_ok()
            );
        }
    }
}
