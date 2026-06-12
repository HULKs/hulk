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
}

impl OdometryScratch {
    pub fn new() -> Self {
        Self {
            correspondences: Vec::with_capacity(NUM_KEYPOINTS),
            inlier_correspondences: Vec::with_capacity(NUM_KEYPOINTS),
            pnp_world_points: Vec::with_capacity(NUM_KEYPOINTS),
            pnp_image_points: Vec::with_capacity(NUM_KEYPOINTS),
            right_observations_by_left_index: vec![None; NUM_KEYPOINTS],
        }
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
        if let Some(observation) = right_observation {
            weight = weight.min(disparity_weight(observation.disparity, parameters));
        }
        scratch.correspondences.push(PoseCorrespondence {
            world_point: previous_point.position,
            image_point: current_pixel,
            right_image_point: right_observation.map(|observation| observation.pixel),
            weight,
        });
    }

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
                    "PnP failed"
                );
                return None;
            }
        };

        if result.inliers.len() < parameters.minimum_pnp_correspondences {
            return None;
        }

        refine_ransac_pose(&result, triangulator, parameters, scratch)
    };

    Some(pnp_pose_to_isometry(&pose))
}

fn estimate_outlier_free_pose(
    triangulator: &StereoTriangulator,
    parameters: &StereoVisualOdometryPoseEstimationParameters,
    scratch: &OdometryScratch,
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

    let refined_pose = refine_pose(&pose, &scratch.correspondences, triangulator, parameters);

    all_reprojection_errors_within_threshold(
        &refined_pose,
        triangulator.intrinsics_f32(),
        parameters,
        scratch,
    )
    .then_some(refined_pose)
}

fn refine_ransac_pose(
    result: &PnPRansacResult,
    triangulator: &StereoTriangulator,
    parameters: &StereoVisualOdometryPoseEstimationParameters,
    scratch: &mut OdometryScratch,
) -> PnPResult {
    scratch.inlier_correspondences.clear();

    for &index in &result.inliers {
        if let Some(correspondence) = scratch.correspondences.get(index).copied() {
            scratch.inlier_correspondences.push(correspondence);
        }
    }

    refine_pose(
        &result.pose,
        &scratch.inlier_correspondences,
        triangulator,
        parameters,
    )
}

fn refine_pose(
    initial_pose: &PnPResult,
    correspondences: &[PoseCorrespondence],
    triangulator: &StereoTriangulator,
    parameters: &StereoVisualOdometryPoseEstimationParameters,
) -> PnPResult {
    if correspondences.len() < parameters.minimum_pnp_correspondences {
        return initial_pose.clone();
    }

    let refined_pose = match refine_pose_lm_direct(
        correspondences,
        triangulator.intrinsics_f32(),
        triangulator.baseline(),
        parameters,
        initial_pose,
    ) {
        Ok(refined_pose) => refined_pose,
        Err(error) => {
            tracing::debug!(
                error,
                correspondences = correspondences.len(),
                "PnP LM refinement failed"
            );
            return initial_pose.clone();
        }
    };

    if is_refinement_better(initial_pose, &refined_pose) {
        refined_pose
    } else {
        tracing::debug!(
            initial_rmse = initial_pose.reproj_rmse,
            refined_rmse = refined_pose.reproj_rmse,
            "PnP LM refinement worsened reprojection error"
        );
        initial_pose.clone()
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

fn all_reprojection_errors_within_threshold(
    pose: &PnPResult,
    intrinsics: &Mat3AF32,
    parameters: &StereoVisualOdometryPoseEstimationParameters,
    scratch: &OdometryScratch,
) -> bool {
    let threshold_squared =
        parameters.ransac_reprojection_threshold_px * parameters.ransac_reprojection_threshold_px;

    scratch.correspondences.iter().all(|correspondence| {
        reprojection_squared_error(
            correspondence.world_point,
            correspondence.image_point,
            pose,
            intrinsics,
        )
        .is_some_and(|error| error <= threshold_squared)
    })
}

fn reprojection_squared_error(
    world_point: Vec3AF32,
    image_point: Vec2F32,
    pose: &PnPResult,
    intrinsics: &Mat3AF32,
) -> Option<f32> {
    let camera_point = vector3_from_vec3a(pose.rotation * world_point + pose.translation);
    let error = residual_with_x_offset(camera_point, image_point, intrinsics, 0.0)?.norm_squared();

    error.is_finite().then_some(error)
}

fn is_refinement_better(ransac_pose: &PnPResult, refined_pose: &PnPResult) -> bool {
    let Some(refined_rmse) = refined_pose.reproj_rmse else {
        return false;
    };
    if !refined_rmse.is_finite() {
        return false;
    }

    match ransac_pose.reproj_rmse {
        Some(ransac_rmse) if ransac_rmse.is_finite() => refined_rmse <= ransac_rmse,
        _ => true,
    }
}

fn pnp_pose_to_isometry(pose: &PnPResult) -> na::Isometry3<f32> {
    let rotation = na::Rotation3::from_matrix_unchecked(matrix3_from_mat3a(&pose.rotation));
    na::Isometry3::from_parts(
        na::Translation3::new(pose.translation.x, pose.translation.y, pose.translation.z),
        na::UnitQuaternion::from_rotation_matrix(&rotation),
    )
}
