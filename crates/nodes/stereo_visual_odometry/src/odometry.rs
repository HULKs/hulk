use kornia_3d::pnp::{
    PnPMethod, PnPRansacResult, PnPResult, RansacParams, solve_pnp, solve_pnp_ransac,
};
use kornia_algebra::{Mat3AF32, Vec2F32, Vec3AF32};
use nalgebra as na;

use crate::{
    feature_extractor::{CurrentLeft, FrameFeatures, Matches, NUM_KEYPOINTS, PreviousLeft},
    parameters::StereoVisualOdometryPoseEstimationParameters,
    pose_refinement::{
        matrix3_from_mat3a, refine_pose_lm_direct, residual_with_x_offset, vector3_from_vec3a,
    },
    triangulator::{StereoPoint, StereoTriangulator},
};

#[derive(Clone, Copy)]
struct PreviousPoint {
    position: Vec3AF32,
    disparity: f32,
}

#[derive(Clone, Copy)]
struct RightImageObservation {
    pixel: Vec2F32,
    disparity: f32,
}

#[derive(Clone, Copy)]
pub(crate) struct PoseCorrespondence {
    pub(crate) world_point: Vec3AF32,
    pub(crate) image_point: Vec2F32,
    pub(crate) right_image_point: Option<Vec2F32>,
    pub(crate) weight: f32,
    pub(crate) right_weight: f32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct OdometryDiagnostics {
    pub correspondences: usize,
    pub left_ransac_inliers: usize,
    pub right_observations: usize,
    pub trusted_right_observations: usize,
    pub used_outlier_free_pose: bool,
    pub refit_used: bool,
    pub lm_attempted: bool,
    pub lm_success: bool,
    pub lm_accepted: bool,
    pub lm_delta_translation_m: Option<f32>,
    pub lm_delta_rotation_deg: Option<f32>,
    pub left_rmse_before_lm: Option<f32>,
    pub right_rmse_before_lm: Option<f32>,
    pub stereo_rmse_before_lm: Option<f32>,
    pub weighted_cost_before_lm: Option<f32>,
    pub left_rmse_after_lm: Option<f32>,
    pub right_rmse_after_lm: Option<f32>,
    pub stereo_rmse_after_lm: Option<f32>,
    pub weighted_cost_after_lm: Option<f32>,
    pub right_bad_fraction_before_lm: Option<f32>,
    pub right_bad_fraction_after_lm: Option<f32>,
}

pub struct PreviousFrame {
    points_by_left_index: Vec<Option<PreviousPoint>>,
}

pub struct OdometryScratch {
    correspondences: Vec<PoseCorrespondence>,
    inlier_correspondences: Vec<PoseCorrespondence>,
    pnp_world_points: Vec<Vec3AF32>,
    pnp_image_points: Vec<Vec2F32>,
    right_observations_by_left_index: Vec<Option<RightImageObservation>>,
    diagnostics: OdometryDiagnostics,
}

impl OdometryScratch {
    pub fn new() -> Self {
        Self {
            correspondences: Vec::with_capacity(NUM_KEYPOINTS),
            inlier_correspondences: Vec::with_capacity(NUM_KEYPOINTS),
            pnp_world_points: Vec::with_capacity(NUM_KEYPOINTS),
            pnp_image_points: Vec::with_capacity(NUM_KEYPOINTS),
            right_observations_by_left_index: vec![None; NUM_KEYPOINTS],
            diagnostics: OdometryDiagnostics::default(),
        }
    }

    pub fn diagnostics(&self) -> OdometryDiagnostics {
        self.diagnostics
    }
}

#[derive(Clone, Copy)]
struct ReprojectionMetrics {
    left_count: usize,
    right_count: usize,
    stereo_count: usize,
    left_squared_error: f32,
    right_squared_error: f32,
    stereo_squared_error: f32,
    weighted_joint_cost: f32,
    right_bad_count: usize,
}

const RIGHT_VALIDATION_THRESHOLD_PX: f32 = 12.0;
const RIGHT_MAX_BAD_FRACTION: f32 = 0.30;
const MAX_LEFT_RMSE_REGRESSION_RATIO: f32 = 1.10;

// Conservative stereo weighting: left temporal reprojection stays the anchor,
// while clean right observations add a soft pull during LM and acceptance.
const RIGHT_MIN_DISPARITY_PX: f32 = 2.0;
const RIGHT_MAX_VERTICAL_ERROR_PX: f32 = 1.5;
const RIGHT_DISPARITY_FULL_WEIGHT_PX: f32 = 20.0;
const RIGHT_MIN_WEIGHT: f32 = 0.05;
const RIGHT_VERTICAL_SIGMA_PX: f32 = 1.0;
const RIGHT_SOFT_COST_ALPHA: f32 = 0.25;
const RIGHT_SIGMA_PX: f32 = 1.5;

impl ReprojectionMetrics {
    fn left_rmse(self) -> Option<f32> {
        (self.left_count > 0 && self.left_squared_error.is_finite())
            .then_some((self.left_squared_error / self.left_count as f32).sqrt())
    }

    fn right_rmse(self) -> Option<f32> {
        (self.right_count > 0 && self.right_squared_error.is_finite())
            .then_some((self.right_squared_error / self.right_count as f32).sqrt())
    }

    fn stereo_rmse(self) -> Option<f32> {
        (self.stereo_count > 0 && self.stereo_squared_error.is_finite())
            .then_some((self.stereo_squared_error / self.stereo_count as f32).sqrt())
    }

    fn right_bad_fraction(self) -> Option<f32> {
        (self.right_count > 0).then_some(self.right_bad_count as f32 / self.right_count as f32)
    }

    fn weighted_cost(self) -> Option<f32> {
        self.weighted_joint_cost
            .is_finite()
            .then_some(self.weighted_joint_cost)
    }
}

impl PreviousFrame {
    pub fn from_stereo_points(points: &[StereoPoint]) -> Self {
        let mut points_by_left_index = vec![None; NUM_KEYPOINTS];
        fill_points_by_left_index(&mut points_by_left_index, points);

        Self {
            points_by_left_index,
        }
    }

    pub fn replace_stereo_points(&mut self, points: &[StereoPoint]) {
        self.points_by_left_index.fill(None);
        fill_points_by_left_index(&mut self.points_by_left_index, points);
    }

    fn point(&self, index: usize) -> Option<PreviousPoint> {
        self.points_by_left_index.get(index).copied().flatten()
    }
}

fn fill_points_by_left_index(
    points_by_left_index: &mut [Option<PreviousPoint>],
    points: &[StereoPoint],
) {
    for point in points {
        if let Some(slot) = points_by_left_index.get_mut(point.left_index) {
            *slot = Some(PreviousPoint {
                position: point.position,
                disparity: point.disparity,
            });
        }
    }
}

fn fill_right_observations_by_left_index(points: &[StereoPoint], scratch: &mut OdometryScratch) {
    scratch.right_observations_by_left_index.fill(None);

    for point in points {
        if let Some(slot) = scratch
            .right_observations_by_left_index
            .get_mut(point.left_index)
        {
            *slot = Some(RightImageObservation {
                pixel: point.right_pixel,
                disparity: point.disparity,
            });
        }
    }
}

fn fill_pnp_inputs(
    correspondences: &[PoseCorrespondence],
    world_points: &mut Vec<Vec3AF32>,
    image_points: &mut Vec<Vec2F32>,
) {
    world_points.clear();
    image_points.clear();

    for correspondence in correspondences {
        world_points.push(correspondence.world_point);
        image_points.push(correspondence.image_point);
    }
}

pub fn estimate_previous_to_current(
    previous: &PreviousFrame,
    current_left: &FrameFeatures<'_, CurrentLeft>,
    current_points: &[StereoPoint],
    temporal_matches: &Matches<'_, PreviousLeft, CurrentLeft>,
    triangulator: &StereoTriangulator,
    parameters: &StereoVisualOdometryPoseEstimationParameters,
    scratch: &mut OdometryScratch,
) -> Option<na::Isometry3<f32>> {
    scratch.diagnostics = OdometryDiagnostics::default();
    scratch.correspondences.clear();
    fill_right_observations_by_left_index(current_points, scratch);

    for (previous_index, current_index, _score) in temporal_matches.left_to_right() {
        if !current_left.is_valid(current_index) {
            continue;
        }
        let Some(previous_point) = previous.point(previous_index) else {
            continue;
        };
        let Some(current_keypoint) = current_left.keypoint(current_index) else {
            continue;
        };
        let current_pixel = triangulator.left_pixel(current_keypoint);
        let right_observation = scratch
            .right_observations_by_left_index
            .get(current_index)
            .copied()
            .flatten();

        let mut weight = disparity_weight(previous_point.disparity, parameters);
        let mut right_weight = 0.0;
        if let Some(observation) = right_observation {
            weight = weight.min(disparity_weight(observation.disparity, parameters));
            right_weight = right_observation_weight(current_pixel, observation);
        }
        scratch.correspondences.push(PoseCorrespondence {
            world_point: previous_point.position,
            image_point: current_pixel,
            right_image_point: right_observation.map(|observation| observation.pixel),
            weight,
            right_weight,
        });
    }

    scratch.diagnostics.correspondences = scratch.correspondences.len();
    scratch.diagnostics.right_observations = scratch
        .correspondences
        .iter()
        .filter(|correspondence| correspondence.right_image_point.is_some())
        .count();
    scratch.diagnostics.trusted_right_observations = scratch
        .correspondences
        .iter()
        .filter(|correspondence| correspondence.right_weight > 0.0)
        .count();

    if scratch.correspondences.len() < parameters.minimum_pnp_correspondences {
        return None;
    }
    fill_pnp_inputs(
        &scratch.correspondences,
        &mut scratch.pnp_world_points,
        &mut scratch.pnp_image_points,
    );

    let pose = if let Some(pose) = estimate_outlier_free_pose(triangulator, parameters, scratch) {
        pose
    } else {
        estimate_ransac_pose(triangulator, parameters, scratch)?
    };

    Some(pnp_pose_to_isometry(&pose))
}

fn right_observation_weight(left_pixel: Vec2F32, observation: RightImageObservation) -> f32 {
    let disparity = left_pixel.x - observation.pixel.x;
    let vertical_error = (left_pixel.y - observation.pixel.y).abs();
    if !disparity.is_finite()
        || !vertical_error.is_finite()
        || disparity <= RIGHT_MIN_DISPARITY_PX
        || vertical_error > RIGHT_MAX_VERTICAL_ERROR_PX
    {
        return 0.0;
    }

    let disparity_weight = (disparity / RIGHT_DISPARITY_FULL_WEIGHT_PX)
        .powi(2)
        .clamp(RIGHT_MIN_WEIGHT, 1.0);
    let vertical_weight = (-0.5 * (vertical_error / RIGHT_VERTICAL_SIGMA_PX).powi(2)).exp();

    RIGHT_SOFT_COST_ALPHA * disparity_weight * vertical_weight / (RIGHT_SIGMA_PX * RIGHT_SIGMA_PX)
}

fn estimate_outlier_free_pose(
    triangulator: &StereoTriangulator,
    parameters: &StereoVisualOdometryPoseEstimationParameters,
    scratch: &mut OdometryScratch,
) -> Option<PnPResult> {
    let pose = match solve_pnp(
        &scratch.pnp_world_points,
        &scratch.pnp_image_points,
        triangulator.intrinsics_f32(),
        None,
        PnPMethod::EPnPDefault,
    ) {
        Ok(pose) => pose,
        Err(error) => {
            tracing::trace!(?error, "all-correspondence PnP failed");
            return None;
        }
    };

    if !pose
        .reproj_rmse
        .is_some_and(|rmse| rmse.is_finite() && rmse <= parameters.ransac_reprojection_threshold_px)
    {
        return None;
    }

    scratch.diagnostics.used_outlier_free_pose = true;
    let mut diagnostics = scratch.diagnostics;
    let pose = refine_pose(
        &pose,
        &scratch.correspondences,
        triangulator,
        parameters,
        true,
        &mut diagnostics,
    );
    scratch.diagnostics = diagnostics;
    pose
}

fn estimate_ransac_pose(
    triangulator: &StereoTriangulator,
    parameters: &StereoVisualOdometryPoseEstimationParameters,
    scratch: &mut OdometryScratch,
) -> Option<PnPResult> {
    let params = RansacParams {
        max_iterations: parameters.ransac_max_iterations,
        reproj_threshold_px: parameters.ransac_reprojection_threshold_px,
        confidence: parameters.ransac_confidence,
        random_seed: None,
        refine: false,
    };
    let result = match solve_pnp_ransac(
        &scratch.pnp_world_points,
        &scratch.pnp_image_points,
        triangulator.intrinsics_f32(),
        None,
        PnPMethod::EPnPDefault,
        &params,
    ) {
        Ok(result) => result,
        Err(error) => {
            tracing::debug!(
                ?error,
                correspondences = scratch.correspondences.len(),
                "left PnP RANSAC failed"
            );
            return None;
        }
    };

    if result.inliers.len() < parameters.minimum_pnp_correspondences {
        return None;
    }

    collect_left_inlier_correspondences(&result, scratch);
    scratch.diagnostics.left_ransac_inliers = scratch.inlier_correspondences.len();

    let ransac_metrics = reprojection_metrics(
        &result.pose,
        &scratch.inlier_correspondences,
        triangulator.intrinsics_f32(),
        triangulator.baseline(),
    );
    fill_pnp_inputs(
        &scratch.inlier_correspondences,
        &mut scratch.pnp_world_points,
        &mut scratch.pnp_image_points,
    );
    let refit_pose = solve_pnp(
        &scratch.pnp_world_points,
        &scratch.pnp_image_points,
        triangulator.intrinsics_f32(),
        None,
        PnPMethod::EPnPDefault,
    )
    .ok();
    let refit_metrics = refit_pose.as_ref().and_then(|pose| {
        reprojection_metrics(
            pose,
            &scratch.inlier_correspondences,
            triangulator.intrinsics_f32(),
            triangulator.baseline(),
        )
    });

    if let (Some(refit_pose), Some(refit_metrics), Some(ransac_metrics)) =
        (refit_pose.as_ref(), refit_metrics, ransac_metrics)
        && refit_is_left_consistent(ransac_metrics, refit_metrics)
    {
        scratch.diagnostics.refit_used = true;
        let mut diagnostics = scratch.diagnostics;
        let pose = refine_pose(
            refit_pose,
            &scratch.inlier_correspondences,
            triangulator,
            parameters,
            true,
            &mut diagnostics,
        );
        scratch.diagnostics = diagnostics;
        return pose;
    }

    let mut diagnostics = scratch.diagnostics;
    let pose = refine_pose(
        &result.pose,
        &scratch.inlier_correspondences,
        triangulator,
        parameters,
        false,
        &mut diagnostics,
    );
    scratch.diagnostics = diagnostics;
    pose
}

fn collect_left_inlier_correspondences(result: &PnPRansacResult, scratch: &mut OdometryScratch) {
    scratch.inlier_correspondences.clear();
    for &index in &result.inliers {
        if let Some(correspondence) = scratch.correspondences.get(index).copied() {
            scratch.inlier_correspondences.push(correspondence);
        }
    }
}

fn refine_pose(
    initial_pose: &PnPResult,
    correspondences: &[PoseCorrespondence],
    triangulator: &StereoTriangulator,
    parameters: &StereoVisualOdometryPoseEstimationParameters,
    allow_initial_fallback: bool,
    diagnostics: &mut OdometryDiagnostics,
) -> Option<PnPResult> {
    if correspondences.len() < parameters.minimum_pnp_correspondences {
        return allow_initial_fallback
            .then(|| pose_with_metrics(initial_pose, correspondences, triangulator))?;
    }

    let initial_metrics = reprojection_metrics(
        initial_pose,
        correspondences,
        triangulator.intrinsics_f32(),
        triangulator.baseline(),
    )?;
    let initial_pose = with_stereo_rmse(initial_pose.clone(), initial_metrics);
    diagnostics.lm_attempted = true;
    fill_diagnostics_before_lm(diagnostics, initial_metrics);
    let refined_pose = match refine_pose_lm_direct(
        correspondences,
        triangulator.intrinsics_f32(),
        triangulator.baseline(),
        parameters,
        &initial_pose,
    ) {
        Ok(refined_pose) => refined_pose,
        Err(error) => {
            tracing::debug!(
                error,
                correspondences = correspondences.len(),
                "PnP LM refinement failed"
            );
            return allow_initial_fallback.then_some(initial_pose);
        }
    };
    let refined_metrics = reprojection_metrics(
        &refined_pose,
        correspondences,
        triangulator.intrinsics_f32(),
        triangulator.baseline(),
    );

    match refined_metrics {
        Some(refined_metrics)
            if is_refinement_better(initial_metrics, refined_metrics)
                && passes_soft_stereo_validation(
                    refined_metrics,
                    parameters.minimum_pnp_correspondences,
                ) =>
        {
            diagnostics.lm_success = true;
            diagnostics.lm_accepted = true;
            fill_diagnostics_after_lm(diagnostics, refined_metrics);
            fill_lm_delta(diagnostics, &initial_pose, &refined_pose);
            Some(with_stereo_rmse(refined_pose, refined_metrics))
        }
        refined_metrics => {
            if let Some(refined_metrics) = refined_metrics {
                diagnostics.lm_success = true;
                fill_diagnostics_after_lm(diagnostics, refined_metrics);
                fill_lm_delta(diagnostics, &initial_pose, &refined_pose);
            }
            tracing::debug!(
                initial_left_rmse = initial_metrics.left_rmse(),
                initial_stereo_rmse = initial_metrics.stereo_rmse(),
                refined_left_rmse = refined_metrics.and_then(|metrics| metrics.left_rmse()),
                refined_stereo_rmse = refined_metrics.and_then(|metrics| metrics.stereo_rmse()),
                "stereo LM failed validation or worsened accepted reprojection metrics"
            );
            allow_initial_fallback.then_some(initial_pose)
        }
    }
}

fn disparity_weight(
    disparity: f32,
    parameters: &StereoVisualOdometryPoseEstimationParameters,
) -> f32 {
    if !disparity.is_finite() || disparity <= 0.0 {
        return parameters.min_disparity_weight;
    }

    (disparity / parameters.full_weight_disparity_px).clamp(parameters.min_disparity_weight, 1.0)
}

fn pose_with_metrics(
    pose: &PnPResult,
    correspondences: &[PoseCorrespondence],
    triangulator: &StereoTriangulator,
) -> Option<PnPResult> {
    let metrics = reprojection_metrics(
        pose,
        correspondences,
        triangulator.intrinsics_f32(),
        triangulator.baseline(),
    )?;
    Some(with_stereo_rmse(pose.clone(), metrics))
}

fn reprojection_metrics(
    pose: &PnPResult,
    correspondences: &[PoseCorrespondence],
    intrinsics: &Mat3AF32,
    baseline: f32,
) -> Option<ReprojectionMetrics> {
    let right_bad_threshold_squared = RIGHT_VALIDATION_THRESHOLD_PX * RIGHT_VALIDATION_THRESHOLD_PX;
    let mut metrics = ReprojectionMetrics {
        left_count: 0,
        right_count: 0,
        stereo_count: 0,
        left_squared_error: 0.0,
        right_squared_error: 0.0,
        stereo_squared_error: 0.0,
        weighted_joint_cost: 0.0,
        right_bad_count: 0,
    };

    for correspondence in correspondences {
        let camera_point =
            vector3_from_vec3a(pose.rotation * correspondence.world_point + pose.translation);
        let left_squared_error =
            residual_with_x_offset(camera_point, correspondence.image_point, intrinsics, 0.0)?
                .norm_squared();
        metrics.left_count += 1;
        metrics.stereo_count += 1;
        metrics.left_squared_error += left_squared_error;
        metrics.stereo_squared_error += left_squared_error;
        metrics.weighted_joint_cost += correspondence.weight * left_squared_error;

        if let Some(right_image_point) = correspondence.right_image_point
            && correspondence.right_weight > 0.0
        {
            let right_squared_error =
                residual_with_x_offset(camera_point, right_image_point, intrinsics, -baseline)?
                    .norm_squared();
            metrics.right_count += 1;
            metrics.stereo_count += 1;
            metrics.right_squared_error += right_squared_error;
            metrics.stereo_squared_error += right_squared_error;
            metrics.weighted_joint_cost += correspondence.right_weight * right_squared_error;
            if right_squared_error > right_bad_threshold_squared {
                metrics.right_bad_count += 1;
            }
        }
    }

    (metrics.left_squared_error.is_finite()
        && metrics.right_squared_error.is_finite()
        && metrics.stereo_squared_error.is_finite()
        && metrics.weighted_joint_cost.is_finite())
    .then_some(metrics)
}

fn with_stereo_rmse(mut pose: PnPResult, metrics: ReprojectionMetrics) -> PnPResult {
    pose.reproj_rmse = metrics.stereo_rmse();
    pose
}

fn fill_diagnostics_before_lm(diagnostics: &mut OdometryDiagnostics, metrics: ReprojectionMetrics) {
    diagnostics.left_rmse_before_lm = metrics.left_rmse();
    diagnostics.right_rmse_before_lm = metrics.right_rmse();
    diagnostics.stereo_rmse_before_lm = metrics.stereo_rmse();
    diagnostics.weighted_cost_before_lm = metrics.weighted_cost();
    diagnostics.right_bad_fraction_before_lm = metrics.right_bad_fraction();
}

fn fill_diagnostics_after_lm(diagnostics: &mut OdometryDiagnostics, metrics: ReprojectionMetrics) {
    diagnostics.left_rmse_after_lm = metrics.left_rmse();
    diagnostics.right_rmse_after_lm = metrics.right_rmse();
    diagnostics.stereo_rmse_after_lm = metrics.stereo_rmse();
    diagnostics.weighted_cost_after_lm = metrics.weighted_cost();
    diagnostics.right_bad_fraction_after_lm = metrics.right_bad_fraction();
}

fn fill_lm_delta(
    diagnostics: &mut OdometryDiagnostics,
    initial_pose: &PnPResult,
    refined_pose: &PnPResult,
) {
    let initial = pnp_pose_to_isometry(initial_pose);
    let refined = pnp_pose_to_isometry(refined_pose);
    let delta = refined * initial.inverse();
    diagnostics.lm_delta_translation_m = Some(delta.translation.vector.norm());
    diagnostics.lm_delta_rotation_deg = Some(delta.rotation.angle().to_degrees());
}

fn passes_soft_stereo_validation(
    metrics: ReprojectionMetrics,
    minimum_right_observations: usize,
) -> bool {
    if metrics.right_count < minimum_right_observations {
        return true;
    }

    let right_rmse_is_bad = metrics
        .right_rmse()
        .is_some_and(|rmse| rmse > RIGHT_VALIDATION_THRESHOLD_PX);
    let right_bad_fraction_is_bad = metrics
        .right_bad_fraction()
        .is_some_and(|fraction| fraction > RIGHT_MAX_BAD_FRACTION);

    !(right_rmse_is_bad && right_bad_fraction_is_bad)
}

fn refit_is_left_consistent(
    ransac_metrics: ReprojectionMetrics,
    refit_metrics: ReprojectionMetrics,
) -> bool {
    let Some(ransac_left_rmse) = ransac_metrics.left_rmse() else {
        return false;
    };
    let Some(refit_left_rmse) = refit_metrics.left_rmse() else {
        return false;
    };

    refit_left_rmse <= ransac_left_rmse * MAX_LEFT_RMSE_REGRESSION_RATIO
}

fn is_refinement_better(initial: ReprojectionMetrics, refined: ReprojectionMetrics) -> bool {
    let Some(initial_weighted_cost) = initial.weighted_cost() else {
        return false;
    };
    let Some(refined_weighted_cost) = refined.weighted_cost() else {
        return false;
    };
    let Some(initial_left_rmse) = initial.left_rmse() else {
        return false;
    };
    let Some(refined_left_rmse) = refined.left_rmse() else {
        return false;
    };

    refined_weighted_cost <= initial_weighted_cost
        && refined_left_rmse <= initial_left_rmse * MAX_LEFT_RMSE_REGRESSION_RATIO
}

fn pnp_pose_to_isometry(pose: &PnPResult) -> na::Isometry3<f32> {
    let rotation = na::Rotation3::from_matrix_unchecked(matrix3_from_mat3a(&pose.rotation));
    na::Isometry3::from_parts(
        na::Translation3::new(pose.translation.x, pose.translation.y, pose.translation.z),
        na::UnitQuaternion::from_rotation_matrix(&rotation),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn intrinsics() -> Mat3AF32 {
        Mat3AF32::from_cols(
            Vec3AF32::new(100.0, 0.0, 0.0),
            Vec3AF32::new(0.0, 100.0, 0.0),
            Vec3AF32::new(0.0, 0.0, 1.0),
        )
    }

    fn identity_pose(rmse: Option<f32>) -> PnPResult {
        PnPResult {
            rotation: Mat3AF32::from_cols(
                Vec3AF32::new(1.0, 0.0, 0.0),
                Vec3AF32::new(0.0, 1.0, 0.0),
                Vec3AF32::new(0.0, 0.0, 1.0),
            ),
            translation: Vec3AF32::new(0.0, 0.0, 0.0),
            rvec: Vec3AF32::new(0.0, 0.0, 0.0),
            reproj_rmse: rmse,
            num_iterations: None,
            converged: None,
        }
    }

    fn correspondence(right_image_point: Option<Vec2F32>) -> PoseCorrespondence {
        PoseCorrespondence {
            world_point: Vec3AF32::new(0.0, 0.0, 10.0),
            image_point: Vec2F32::new(0.0, 0.0),
            right_image_point,
            weight: 1.0,
            right_weight: right_image_point.map_or(0.0, |_| 1.0),
        }
    }

    #[test]
    fn metrics_mark_good_left_bad_right_correspondence_as_stereo_bad() {
        let pose = identity_pose(None);
        let correspondences = [correspondence(Some(Vec2F32::new(20.0, 0.0)))];

        let metrics = reprojection_metrics(&pose, &correspondences, &intrinsics(), 0.5)
            .expect("metrics should be finite");

        assert_eq!(metrics.left_rmse(), Some(0.0));
        assert_eq!(metrics.right_bad_count, 1);
        assert!(!passes_soft_stereo_validation(metrics, 1));
    }

    #[test]
    fn metrics_accept_matching_left_and_right_correspondence() {
        let pose = identity_pose(None);
        let correspondences = [correspondence(Some(Vec2F32::new(-5.0, 0.0)))];

        let metrics = reprojection_metrics(&pose, &correspondences, &intrinsics(), 0.5)
            .expect("metrics should be finite");

        assert_eq!(metrics.left_count, 1);
        assert_eq!(metrics.right_count, 1);
        assert_eq!(metrics.stereo_rmse(), Some(0.0));
        assert!(passes_soft_stereo_validation(metrics, 1));
    }

    #[test]
    fn metrics_are_left_only_when_right_observation_is_missing() {
        let pose = identity_pose(None);
        let correspondences = [correspondence(None)];

        let metrics = reprojection_metrics(&pose, &correspondences, &intrinsics(), 0.5)
            .expect("metrics should be finite");

        assert_eq!(metrics.left_count, 1);
        assert_eq!(metrics.right_count, 0);
        assert_eq!(metrics.stereo_count, 1);
        assert!(passes_soft_stereo_validation(metrics, 1));
    }

    #[test]
    fn refinement_comparison_requires_stereo_improvement_without_large_left_regression() {
        let initial = ReprojectionMetrics {
            left_count: 1,
            right_count: 1,
            stereo_count: 2,
            left_squared_error: 4.0,
            right_squared_error: 4.0,
            stereo_squared_error: 8.0,
            weighted_joint_cost: 8.0,
            right_bad_count: 0,
        };
        let better = ReprojectionMetrics {
            left_squared_error: 4.2,
            right_squared_error: 1.0,
            stereo_squared_error: 5.2,
            weighted_joint_cost: 5.2,
            ..initial
        };
        let bad_left_regression = ReprojectionMetrics {
            left_squared_error: 9.0,
            right_squared_error: 1.0,
            stereo_squared_error: 10.0,
            weighted_joint_cost: 10.0,
            ..initial
        };

        assert!(is_refinement_better(initial, better));
        assert!(!is_refinement_better(initial, bad_left_regression));
    }
}
