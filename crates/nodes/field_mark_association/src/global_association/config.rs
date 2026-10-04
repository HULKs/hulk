use ros_z::Message;
use serde::{Deserialize, Serialize};

/// Guided gravity-constrained matching and measurement noise shared with tracking.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize, Message)]
#[serde(default, deny_unknown_fields)]
pub struct GlobalAssociationConfig {
    /// Per-axis metric tolerance when matching half-turn landmark partners.
    pub symmetry_epsilon: f32,
    /// Minimum pair distance in unit-height projected coordinates before normalization.
    pub min_pair_distance: f32,
    /// Floor for the squared-edge denominator in normalized seed triangle quality.
    pub min_triangle_denominator: f32,
    /// Maximum supported detections accepted before filtering.
    pub max_input_detections: usize,
    /// Maximum detections retained after filtering and de-duplication.
    pub max_retained_detections: usize,
    /// Highest-ranked inliers considered for a non-collinear consensus check.
    pub seed_pool_size: usize,
    pub min_inliers: usize,
    pub confidence_threshold: f32,
    /// Same-class detections closer than this are treated as duplicates.
    pub duplicate_pixel_distance: f32,
    /// Minimum seed edge length in meters.
    pub min_detection_baseline: f32,
    /// Minimum normalized downward ray component accepted by global matching.
    pub min_downward_ray_fraction: f32,
    /// Minimum normalized triangle quality used for seed selection.
    pub min_seed_quality: f32,
    /// Pixel measurement/calibration floor, shared with image-space tracking.
    pub detection_pixel_sigma: f32,
    /// Shared local roll/pitch uncertainty in radians.
    pub imu_tilt_sigma: f32,
    /// Height uncertainty floor for image-space tracking, in metres. Startup fits
    /// height from landmarks and does not use this parameter.
    pub height_sigma: f32,
    /// Squared bound: global invariant gates use its square root; tracking uses it
    /// directly for 2D residuals and preserves its chi-square tail probability in
    /// joint residuals. This is not an assignment-certification probability.
    pub mahalanobis_gate: f32,
    /// Required fraction of confidence-filtered, deduplicated detections supporting
    /// the winning pose; uncertain-ray pruning does not reduce this denominator.
    pub min_inlier_fraction: f32,
    /// L/T/X class disagreement adds this to the squared pixel Mahalanobis error.
    /// Goalposts and penalty spots are never reclassified.
    pub class_mismatch_penalty: f32,
    /// Required sampled-pose score ratio and fixed-pose assignment separation.
    pub score_ratio: f32,
    /// Optical-center height above the field, not robot-body height.
    pub min_camera_height: f32,
    pub max_camera_height: f32,
    /// Unweighted pixel RMS ceiling for the fitted consensus.
    pub max_rms_px: f32,
    /// Minimum fitted optical depth for global reprojection validation.
    pub min_reprojection_depth: f32,
    /// Per-call work ceiling. Global matching charges proposals, landmark projections,
    /// assignment-row scans, bounded refinement observations and assignment certification.
    /// A row scans the fixed field map. At most 128 hypotheses and 2048 proposals are sampled;
    /// this is not exhaustive assignment certification. Exhaustion rejects the frame.
    /// Joint tracking retains its candidate-extension accounting and exhaustion semantics.
    pub max_work: usize,
}

impl Default for GlobalAssociationConfig {
    fn default() -> Self {
        Self {
            symmetry_epsilon: 1.0e-4,
            min_pair_distance: 1.0e-6,
            min_triangle_denominator: 1.0e-12,
            max_input_detections: 128,
            max_retained_detections: 32,
            seed_pool_size: 8,
            min_inliers: 3,
            confidence_threshold: 0.35,
            duplicate_pixel_distance: 1.0,
            min_detection_baseline: 0.25,
            min_downward_ray_fraction: 1.0e-4,
            min_seed_quality: 1.0e-5,
            detection_pixel_sigma: 2.0,
            imu_tilt_sigma: 0.02,
            height_sigma: 0.02,
            mahalanobis_gate: 9.21,
            min_inlier_fraction: 0.7,
            class_mismatch_penalty: 4.0,
            score_ratio: 1.05,
            min_camera_height: 0.25,
            max_camera_height: 2.0,
            max_rms_px: 10.0,
            min_reprojection_depth: 0.01,
            max_work: 100_000,
        }
    }
}

impl GlobalAssociationConfig {
    pub fn validate(&self) -> Result<(), String> {
        let map_size =
            crate::map::candidate_points(&types::field_dimensions::FieldDimensions::SPL_2025).len();
        if !(3..=map_size).contains(&self.min_inliers) {
            return Err("global_localizer.min_inliers must be between 3 and the map size".into());
        }
        if !(0.0..=1.0).contains(&self.confidence_threshold) {
            return Err("global_localizer.confidence_threshold must be in [0, 1]".into());
        }
        if self.max_retained_detections < 3
            || self.seed_pool_size < 3
            || self.seed_pool_size > self.max_retained_detections
            || self.max_input_detections < self.max_retained_detections
        {
            return Err("invalid global detection limits".into());
        }
        for (name, value) in [
            ("symmetry_epsilon", self.symmetry_epsilon),
            ("min_pair_distance", self.min_pair_distance),
            ("min_triangle_denominator", self.min_triangle_denominator),
            ("min_detection_baseline", self.min_detection_baseline),
            ("duplicate_pixel_distance", self.duplicate_pixel_distance),
            ("min_downward_ray_fraction", self.min_downward_ray_fraction),
            ("min_seed_quality", self.min_seed_quality),
            ("detection_pixel_sigma", self.detection_pixel_sigma),
            ("imu_tilt_sigma", self.imu_tilt_sigma),
            ("height_sigma", self.height_sigma),
            ("mahalanobis_gate", self.mahalanobis_gate),
            ("class_mismatch_penalty", self.class_mismatch_penalty),
            ("min_camera_height", self.min_camera_height),
            ("max_camera_height", self.max_camera_height),
            ("max_rms_px", self.max_rms_px),
            ("min_reprojection_depth", self.min_reprojection_depth),
        ] {
            if !value.is_finite() || value <= 0.0 {
                return Err(format!("global_localizer.{name} must be finite and > 0"));
            }
        }
        if !self.min_inlier_fraction.is_finite()
            || !(0.0..=1.0).contains(&self.min_inlier_fraction)
            || self.min_inlier_fraction == 0.0
            || !self.score_ratio.is_finite()
            || self.score_ratio <= 1.0
            || self.min_camera_height >= self.max_camera_height
        {
            return Err("invalid global consensus, score ratio or camera height limits".into());
        }
        if self.max_work == 0 {
            return Err("global_localizer.max_work must be > 0".into());
        }
        Ok(())
    }
}
