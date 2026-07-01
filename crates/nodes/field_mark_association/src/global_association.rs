use std::time::Duration;

use coordinate_systems::{Field, Ground, Pixel, Robot};
use linear_algebra::{Isometry3, Point2};
use ros_z::Message;
use serde::{Deserialize, Serialize};
mod map;
mod solver;

pub(crate) use solver::{GlobalLocalizationInput, GlobalLocalizationResult};

pub(crate) const FEATURE_CLASSES: [VisualFeatureClass; 5] = [
    VisualFeatureClass::GoalPost,
    VisualFeatureClass::LSpot,
    VisualFeatureClass::TSpot,
    VisualFeatureClass::XSpot,
    VisualFeatureClass::PenaltySpot,
];
pub(crate) const GLOBAL_LOCALIZER_MAX_DETECTIONS: usize = 32;

#[derive(Clone, Debug)]
pub(crate) struct GlobalAssociator {
    config: GlobalAssociationConfig,
}

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
    /// Minimum weighted candidate score for acceptance.
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

/// Configuration for pose-hint nearest-neighbor association fallback.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize, Message)]
#[serde(deny_unknown_fields)]
pub struct PoseHintAssociationConfig {
    /// Enables pose-hint fallback when global uniqueness is unavailable.
    pub enabled: bool,
    /// Maximum accepted age difference between image time and pose hint time.
    pub max_pose_age: Duration,
    /// Maximum reprojection error under the current pose hint.
    pub max_reprojection_error_px: f32,
    /// Minimum pixel gap between the closest and second-closest same-class landmark projection.
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

fn validate_positive_f32(value: f32, message: &str) -> Result<(), String> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err(message.to_string())
    }
}

impl GlobalAssociator {
    /// Creates a global localizer with fixed association and scoring parameters.
    pub fn new(config: GlobalAssociationConfig) -> Self {
        Self { config }
    }

    /// Attempts to localize one frame of field-feature detections against the known field map.
    ///
    /// Returns `None` when no safe stable association set passes the configured gates. Returned
    /// ambiguous results are diagnostic only; only `UniqueModuloSymmetry` results are safe for
    /// backend ingestion.
    pub fn localize(&self, input: GlobalLocalizationInput<'_>) -> Option<GlobalLocalizationResult> {
        solver::solve(input, self.config)
    }

    pub(crate) fn localize_detailed(
        &self,
        input: GlobalLocalizationInput<'_>,
    ) -> Option<GlobalLocalizationDetailedDebug> {
        solver::solve_detailed(input, self.config)
    }

    pub(crate) fn associate_with_pose_hint(
        &self,
        input: GlobalLocalizationInput<'_>,
        config: PoseHintAssociationConfig,
    ) -> PoseHintAssociationResult {
        solver::associate_with_pose_hint(input, self.config, config)
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct PoseHintAssociationResult {
    pub associations: Vec<FeatureAssociation>,
    pub reprojection_rmse: Option<f32>,
}

impl PoseHintAssociationResult {
    pub(crate) fn is_healthy(&self, config: PoseHintAssociationConfig) -> bool {
        self.associations.len() >= config.healthy_min_inliers
            && self
                .reprojection_rmse
                .is_some_and(|rmse| rmse <= config.healthy_max_rmse_px)
    }
}

impl Default for GlobalAssociator {
    fn default() -> Self {
        Self::new(GlobalAssociationConfig::default())
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
/// Field-feature classes supported by the global association solver.
pub enum VisualFeatureClass {
    /// Upright goalpost landmark detected at its field-contact point.
    GoalPost,
    /// L-shaped line crossing landmark.
    LSpot,
    /// T-shaped line crossing landmark.
    TSpot,
    /// X-shaped line crossing landmark.
    XSpot,
    /// Penalty marker landmark.
    PenaltySpot,
}

#[derive(Clone, Debug)]
pub(crate) struct FeatureAssociations {
    pub robot_to_field: Isometry3<Robot, Field>,
    pub features: Vec<FeatureAssociation>,
    pub score: GlobalLocalizationScore,
}

#[derive(Clone, Debug)]
pub(crate) struct FeatureAssociation {
    pub detection_id: usize,
    pub landmark_id: usize,
    pub detection: Point2<Pixel>,
    pub field_point: Point2<Field>,
}

#[derive(Clone, Copy, Debug)]
/// Scores and residual summaries for a global localization hypothesis.
pub struct GlobalLocalizationScore {
    /// Accepted fixed feature association count.
    pub inliers: usize,
    /// Weighted internal candidate score used by `min_score` and `score_ratio`.
    pub candidate_score: f32,
    /// Metric field-space RMS residual used by `rms_threshold`.
    pub metric_rms_residual: f32,
    /// Pixel reprojection RMS for the selected pose and fixed associations.
    pub reprojection_rmse: f32,
    /// Sum of squared pixel reprojection errors for the selected pose.
    pub total_cost: f32,
}

#[derive(Clone, Debug)]
/// Detailed per-feature debug payload for visualizing a global localization result.
pub struct GlobalLocalizationDetailedDebug {
    /// Certification status for the returned associations.
    pub status: GlobalLocalizationDetailedStatus,
    /// Selected robot pose in the field frame.
    pub robot_to_field: Isometry3<Robot, Field>,
    /// Candidate score and residual summary for the selected result.
    pub score: GlobalLocalizationScore,
    /// Back-projected detections considered by the solver.
    pub detections: Vec<GlobalLocalizationDebugDetection>,
    /// Field landmarks projected into the image for the selected pose.
    pub projected_features: Vec<GlobalLocalizationDebugProjection>,
    /// Accepted detection-to-landmark associations for the selected result.
    pub associations: Vec<GlobalLocalizationDebugAssociation>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Certification status for detailed global-localization debug output.
pub enum GlobalLocalizationDetailedStatus {
    /// Returned associations are plausible, but uniqueness was not certified.
    /// This can also mean the bounded search was truncated before all competitors were excluded.
    Ambiguous,
    /// Returned stable associations are unique after quotienting the unavoidable 180 degree field
    /// symmetry.
    UniqueModuloSymmetry,
}

#[derive(Clone, Debug)]
/// Debug record for one field-feature detection considered by global localization.
pub struct GlobalLocalizationDebugDetection {
    /// Stable detection index from the raw feature stream.
    pub index: usize,
    /// Landmark class inferred from the detector output.
    pub class: VisualFeatureClass,
    /// Input image point used by projection and association.
    pub pixel: Point2<Pixel>,
    /// Back-projected ground point under the current camera geometry.
    pub ground: Point2<Ground>,
}

#[derive(Clone, Debug)]
/// Debug record for one known field landmark projected into the current camera image.
pub struct GlobalLocalizationDebugProjection {
    /// Landmark identifier in the generated field map.
    pub index: usize,
    /// Identifier of the landmark reached by 180-degree field symmetry.
    pub symmetric_index: usize,
    /// Landmark class used for same-class association.
    pub class: VisualFeatureClass,
    /// Landmark position in field coordinates.
    pub field_point: Point2<Field>,
    /// Pixel projection for the selected pose, or `None` if the landmark is behind the camera.
    pub projected_pixel: Option<Point2<Pixel>>,
    /// Whether this landmark is part of the accepted association set.
    pub accepted: bool,
}

#[derive(Clone, Debug)]
/// Debug record for one accepted detection-to-landmark association.
pub struct GlobalLocalizationDebugAssociation {
    /// Stable detection index from the raw feature stream.
    pub detection_index: usize,
    /// Landmark identifier in the generated field map.
    pub feature_index: usize,
    /// Landmark class shared by the detection and field point.
    pub class: VisualFeatureClass,
    /// Input detection pixel.
    pub detection_pixel: Point2<Pixel>,
    /// Detection back-projected onto the ground plane.
    pub back_projected_ground: Point2<Ground>,
    /// Associated landmark position in field coordinates.
    pub field_point: Point2<Field>,
    /// Pixel projection of the associated landmark for the selected pose.
    pub projected_pixel: Option<Point2<Pixel>>,
    /// Pixel distance between `detection_pixel` and `projected_pixel`, if projected.
    pub reprojection_error_px: Option<f32>,
}

impl GlobalLocalizationResult {
    /// Returns the best association set carried by this result, regardless of certification status.
    pub fn associations(&self) -> &FeatureAssociations {
        match self {
            Self::Ambiguous(associations) | Self::UniqueModuloSymmetry(associations) => {
                associations
            }
        }
    }

    #[cfg(test)]
    /// Returns whether this test result is certified unique modulo field symmetry.
    pub fn is_unique(&self) -> bool {
        matches!(self, Self::UniqueModuloSymmetry(_))
    }

    pub(crate) fn unique_feature_associations(&self) -> Option<&[FeatureAssociation]> {
        match self {
            Self::UniqueModuloSymmetry(associations) => Some(&associations.features),
            Self::Ambiguous(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use coordinate_systems::{Camera, Field, Ground, Robot};
    use linear_algebra::{IntoTransform, Point2, point};
    use projection::intrinsic::Intrinsic;
    use types::field_dimensions::{FieldDimensions, Half, Side};

    use super::*;
    use crate::{DetectedVisualFeature, DetectedVisualFeatures};

    fn camera_intrinsic() -> Intrinsic {
        Intrinsic::new(nalgebra::vector![100.0, 100.0], point![320.0, 240.0])
    }

    fn robot_to_camera() -> Isometry3<Robot, Camera> {
        nalgebra::Isometry3::translation(0.0, 0.0, 0.5).framed_transform()
    }

    fn input<'a>(
        visual_features: &'a DetectedVisualFeatures,
        field_dimensions: &'a FieldDimensions,
        config_hint: Option<Isometry3<Robot, Field>>,
    ) -> GlobalLocalizationInput<'a> {
        GlobalLocalizationInput {
            visual_features,
            field_dimensions,
            ground_to_robot: Isometry3::<Ground, Robot>::identity(),
            robot_to_camera: robot_to_camera(),
            camera_intrinsic: camera_intrinsic(),
            pose_hint: config_hint,
        }
    }

    fn project_point(point: Point2<Field>) -> DetectedVisualFeature {
        let field_to_camera = robot_to_camera() * Isometry3::<Field, Robot>::identity();
        let pixel = camera_intrinsic().project((field_to_camera * point.extend(0.0)).coords());
        DetectedVisualFeature {
            pixel,
            confidence: 0.9,
        }
    }

    fn synthetic_features(field: &FieldDimensions) -> DetectedVisualFeatures {
        DetectedVisualFeatures {
            goalposts: vec![
                project_point(field.goal_post(Half::Opponent, Side::Left)),
                project_point(field.goal_post(Half::Opponent, Side::Right)),
            ],
            l_spots: vec![project_point(field.corner(Half::Opponent, Side::Left))],
            x_spots: Vec::new(),
            t_spots: vec![project_point(field.t_crossing(Side::Left))],
            penalty_spots: vec![project_point(field.penalty_spot(Half::Opponent))],
        }
    }

    #[test]
    fn pose_hint_fallback_associates_single_feature() {
        let field = FieldDimensions::SPL_2025;
        let penalty_spot = field.penalty_spot(Half::Opponent);
        let features = DetectedVisualFeatures {
            penalty_spots: vec![project_point(penalty_spot)],
            ..Default::default()
        };
        let localizer = GlobalAssociator::default();

        let associations = localizer.associate_with_pose_hint(
            input(
                &features,
                &field,
                Some(Isometry3::<Robot, Field>::identity()),
            ),
            PoseHintAssociationConfig::default(),
        );

        assert_eq!(associations.associations.len(), 1);
        assert_eq!(associations.associations[0].field_point, penalty_spot);
    }

    #[test]
    fn pose_hint_fallback_rejects_second_best_tie() {
        let field = FieldDimensions::SPL_2025;
        let features = DetectedVisualFeatures {
            penalty_spots: vec![project_point(point![<Field>, 0.0, 0.0])],
            ..Default::default()
        };
        let localizer = GlobalAssociator::default();

        let associations = localizer.associate_with_pose_hint(
            input(
                &features,
                &field,
                Some(Isometry3::<Robot, Field>::identity()),
            ),
            PoseHintAssociationConfig {
                second_best_reprojection_margin_px: 10_000.0,
                ..Default::default()
            },
        );

        assert!(associations.associations.is_empty());
    }

    #[test]
    fn recovers_mixed_feature_associations_with_static_height_gate() -> Result<(), String> {
        let field = FieldDimensions::SPL_2025;
        let features = synthetic_features(&field);
        let localizer = GlobalAssociator::default();

        let Some(result) = localizer.localize(input(
            &features,
            &field,
            Some(Isometry3::<Robot, Field>::identity()),
        )) else {
            return Err("synthetic features should localize".to_string());
        };

        assert!(result.is_unique());
        assert_eq!(result.associations().features.len(), 5);
        assert!(result.associations().score.reprojection_rmse < 1.0e-3);
        Ok(())
    }

    #[test]
    fn rejects_static_height_outside_gate() {
        let field = FieldDimensions::SPL_2025;
        let features = synthetic_features(&field);
        let localizer = GlobalAssociator::new(GlobalAssociationConfig {
            height_max: 0.4,
            association_gate: 0.01,
            ..Default::default()
        });

        assert!(
            localizer
                .localize(input(
                    &features,
                    &field,
                    Some(Isometry3::<Robot, Field>::identity())
                ))
                .is_none()
        );
    }

    #[test]
    fn rejects_low_confidence_detections() {
        let field = FieldDimensions::SPL_2025;
        let mut features = synthetic_features(&field);
        for feature in features
            .goalposts
            .iter_mut()
            .chain(features.l_spots.iter_mut())
            .chain(features.t_spots.iter_mut())
            .chain(features.x_spots.iter_mut())
            .chain(features.penalty_spots.iter_mut())
        {
            feature.confidence = 0.1;
        }
        let localizer = GlobalAssociator::default();

        assert!(
            localizer
                .localize(input(
                    &features,
                    &field,
                    Some(Isometry3::<Robot, Field>::identity())
                ))
                .is_none()
        );
    }
}
