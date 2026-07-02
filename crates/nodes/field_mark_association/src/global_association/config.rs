use std::time::Duration;

use ros_z::Message;
use serde::{Deserialize, Serialize};

pub(crate) const GLOBAL_LOCALIZER_MAX_DETECTIONS: usize = 32;

/// Configuration for global field-feature association and pose recovery.
///
/// These parameters gate detections, candidate associations, and uniqueness certification before
/// any visual associations are exposed to the backend as fixed reprojection factors.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize, Message)]
#[serde(deny_unknown_fields)]
pub struct GlobalAssociationConfig {
    /// Minimum accepted fixed associations for any published result.
    pub min_inliers: usize,
    /// Minimum detector confidence for a field-feature detection.
    pub min_confidence: f32,
    /// Minimum normalized-ray baseline between two detections.
    pub min_detection_baseline: f32,
    /// Minimum metric baseline between two map landmarks.
    pub min_map_baseline: f32,
    /// Lower plausible camera-height scale for global association hypotheses.
    pub height_min: f32,
    /// Upper plausible camera-height scale for global association hypotheses.
    pub height_max: f32,
    /// Maximum metric distance from a predicted landmark to an accepted same-class landmark.
    pub association_gate: f32,
    /// Maximum RMS metric association residual for an accepted candidate.
    pub rms_threshold: f32,
    /// Minimum weighted internal candidate score used by acceptance.
    pub min_score: f32,
    /// Minimum best-to-second-best non-equivalent score ratio.
    pub score_ratio: f32,
    /// Metric residual penalty in the candidate score.
    pub residual_weight: f32,
}

impl Default for GlobalAssociationConfig {
    fn default() -> Self {
        Self {
            min_inliers: 5,
            min_confidence: 0.3,
            min_detection_baseline: 0.1,
            min_map_baseline: 0.25,
            height_min: 0.2,
            height_max: 1.0,
            association_gate: 0.6,
            rms_threshold: 0.45,
            min_score: 0.0,
            score_ratio: 1.05,
            residual_weight: 0.1,
        }
    }
}

impl GlobalAssociationConfig {
    /// Validates that global-localizer parameters are finite, positive where required, and
    /// consistent with the solver's fixed detection cap.
    pub fn validate(&self) -> Result<(), String> {
        if self.min_inliers < 3 {
            return Err("global_localizer.min_inliers must be at least 3".to_string());
        }
        if self.min_inliers > GLOBAL_LOCALIZER_MAX_DETECTIONS {
            return Err(format!(
                "global_localizer.min_inliers must be <= {GLOBAL_LOCALIZER_MAX_DETECTIONS} \
                     because the solver caps detections"
            ));
        }
        if !self.min_confidence.is_finite()
            || self.min_confidence < 0.0
            || self.min_confidence > 1.0
        {
            return Err("global_localizer.min_confidence must be finite and in [0, 1]".to_string());
        }
        validate_positive_f32(
            self.min_detection_baseline,
            "global_localizer.min_detection_baseline must be finite and > 0",
        )?;
        validate_positive_f32(
            self.min_map_baseline,
            "global_localizer.min_map_baseline must be finite and > 0",
        )?;
        validate_positive_f32(
            self.height_min,
            "global_localizer.height_min must be finite and > 0",
        )?;
        validate_positive_f32(
            self.height_max,
            "global_localizer.height_max must be finite and > 0",
        )?;
        if self.height_min > self.height_max {
            return Err("global_localizer.height_min must be <= height_max".to_string());
        }
        validate_positive_f32(
            self.association_gate,
            "global_localizer.association_gate must be finite and > 0",
        )?;
        validate_positive_f32(
            self.rms_threshold,
            "global_localizer.rms_threshold must be finite and > 0",
        )?;
        if !self.min_score.is_finite() || self.min_score < 0.0 {
            return Err("global_localizer.min_score must be finite and >= 0".to_string());
        }
        if !self.score_ratio.is_finite() || self.score_ratio < 1.0 {
            return Err("global_localizer.score_ratio must be finite and >= 1".to_string());
        }
        if !self.residual_weight.is_finite() || self.residual_weight < 0.0 {
            return Err("global_localizer.residual_weight must be finite and >= 0".to_string());
        }
        Ok(())
    }
}

/// Configuration for pose-hint per-class association fallback.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize, Message)]
#[serde(deny_unknown_fields)]
pub struct PoseHintAssociationConfig {
    /// Enables pose-hint fallback when global uniqueness is unavailable.
    pub enabled: bool,
    /// Maximum accepted age difference between image time and pose hint time.
    pub max_pose_age: Duration,
    /// Maximum reprojection error under the current pose hint.
    pub max_reprojection_error_px: f32,
    /// Minimum single-detection pixel gap between closest and second-closest same-class landmark.
    /// Multiple viable same-class detections are disambiguated by joint assignment instead.
    pub second_best_reprojection_margin_px: f32,
    /// Minimum pose-hint associations needed to consider tracking healthy.
    pub healthy_min_inliers: usize,
    /// Maximum pose-hint frame reprojection RMSE for healthy tracking.
    pub healthy_max_rmse_px: f32,
    /// Consecutive agreeing global-localization frames required before recovery is accepted.
    pub recovery_frames: usize,
    /// Maximum translation difference for accepting a global result against a pose hint.
    pub recovery_max_pose_distance: f32,
    /// Maximum yaw difference for accepting a global result against a pose hint.
    pub recovery_max_pose_angle: f32,
    /// Maximum yaw difference for accepting a global recovery on the same symmetry branch.
    pub recovery_max_branch_yaw_error: f32,
}

impl Default for PoseHintAssociationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_pose_age: Duration::from_millis(250),
            max_reprojection_error_px: 80.0,
            second_best_reprojection_margin_px: 15.0,
            healthy_min_inliers: 3,
            healthy_max_rmse_px: 30.0,
            recovery_frames: 5,
            recovery_max_pose_distance: 0.5,
            recovery_max_pose_angle: 15.0_f32.to_radians(),
            recovery_max_branch_yaw_error: 70.0_f32.to_radians(),
        }
    }
}

impl PoseHintAssociationConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.max_pose_age.is_zero() {
            return Err("pose_hint.max_pose_age must be > 0".to_string());
        }
        validate_positive_f32(
            self.max_reprojection_error_px,
            "pose_hint.max_reprojection_error_px must be finite and > 0",
        )?;
        if !self.second_best_reprojection_margin_px.is_finite()
            || self.second_best_reprojection_margin_px < 0.0
        {
            return Err(
                "pose_hint.second_best_reprojection_margin_px must be finite and >= 0".to_string(),
            );
        }
        if self.healthy_min_inliers == 0 {
            return Err("pose_hint.healthy_min_inliers must be > 0".to_string());
        }
        validate_positive_f32(
            self.healthy_max_rmse_px,
            "pose_hint.healthy_max_rmse_px must be finite and > 0",
        )?;
        if self.recovery_frames == 0 {
            return Err("pose_hint.recovery_frames must be > 0".to_string());
        }
        validate_positive_f32(
            self.recovery_max_pose_distance,
            "pose_hint.recovery_max_pose_distance must be finite and > 0",
        )?;
        validate_positive_f32(
            self.recovery_max_pose_angle,
            "pose_hint.recovery_max_pose_angle must be finite and > 0",
        )?;
        if !self.recovery_max_branch_yaw_error.is_finite()
            || self.recovery_max_branch_yaw_error <= 0.0
            || self.recovery_max_branch_yaw_error >= std::f32::consts::FRAC_PI_2
        {
            return Err(
                "pose_hint.recovery_max_branch_yaw_error must be finite and in (0, pi/2)"
                    .to_string(),
            );
        }
        Ok(())
    }
}

fn validate_positive_f32(value: f32, message: &str) -> Result<(), String> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err(message.to_string())
    }
}
