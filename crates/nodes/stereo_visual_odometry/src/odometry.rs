use kornia_3d::pnp::{
    LMRefineParams, PnPMethod, PnPRansacResult, PnPResult, RansacParams, refine_pose_lm,
    solve_pnp_ransac,
};
use kornia_algebra::{Vec2F32, Vec3AF32};
use nalgebra as na;

use crate::{
    feature_extractor::{CurrentLeft, FrameFeatures, KEYPOINTS, Matches, PreviousLeft},
    triangulator::{StereoPoint, StereoTriangulator},
};

const MIN_PNP_CORRESPONDENCES: usize = 8;

pub struct PreviousFrame {
    points_by_left_index: Vec<Option<Vec3AF32>>,
}

pub struct OdometryScratch {
    world_points: Vec<Vec3AF32>,
    image_points: Vec<Vec2F32>,
    inlier_world_points: Vec<Vec3AF32>,
    inlier_image_points: Vec<Vec2F32>,
}

impl OdometryScratch {
    pub fn new() -> Self {
        Self {
            world_points: Vec::with_capacity(KEYPOINTS),
            image_points: Vec::with_capacity(KEYPOINTS),
            inlier_world_points: Vec::with_capacity(KEYPOINTS),
            inlier_image_points: Vec::with_capacity(KEYPOINTS),
        }
    }
}

impl PreviousFrame {
    pub fn from_stereo_points(points: &[StereoPoint]) -> Self {
        let mut points_by_left_index = vec![None; KEYPOINTS];
        fill_points_by_left_index(&mut points_by_left_index, points);

        Self {
            points_by_left_index,
        }
    }

    pub fn replace_stereo_points(&mut self, points: &[StereoPoint]) {
        self.points_by_left_index.fill(None);
        fill_points_by_left_index(&mut self.points_by_left_index, points);
    }

    fn point(&self, index: usize) -> Option<Vec3AF32> {
        self.points_by_left_index.get(index).copied().flatten()
    }
}

fn fill_points_by_left_index(
    points_by_left_index: &mut [Option<Vec3AF32>],
    points: &[StereoPoint],
) {
    for point in points {
        if let Some(slot) = points_by_left_index.get_mut(point.left_index) {
            *slot = Some(point.position);
        }
    }
}

pub fn estimate_previous_to_current(
    previous: &PreviousFrame,
    current_left: &FrameFeatures<'_, CurrentLeft>,
    temporal_matches: &Matches<'_, PreviousLeft, CurrentLeft>,
    triangulator: &StereoTriangulator,
    scratch: &mut OdometryScratch,
) -> Option<na::Isometry3<f32>> {
    scratch.world_points.clear();
    scratch.image_points.clear();

    for (previous_index, current_index, _score) in temporal_matches.left_to_right() {
        if !current_left.is_valid(current_index) {
            continue;
        }
        let Some(world_point) = previous.point(previous_index) else {
            continue;
        };
        let Some(current_keypoint) = current_left.keypoint(current_index) else {
            continue;
        };
        let current_pixel = triangulator.left_pixel(current_keypoint);

        scratch.world_points.push(world_point);
        scratch.image_points.push(current_pixel);
    }

    if scratch.world_points.len() < MIN_PNP_CORRESPONDENCES {
        return None;
    }

    let params = RansacParams {
        max_iterations: 100,
        reproj_threshold_px: 6.0,
        confidence: 0.99,
        random_seed: None,
        refine: false,
    };
    let result = match solve_pnp_ransac(
        &scratch.world_points,
        &scratch.image_points,
        triangulator.intrinsics_f32(),
        None,
        PnPMethod::EPnPDefault,
        &params,
    ) {
        Ok(result) => result,
        Err(error) => {
            tracing::debug!(
                ?error,
                correspondences = scratch.world_points.len(),
                "PnP failed"
            );
            return None;
        }
    };

    if result.inliers.len() < MIN_PNP_CORRESPONDENCES {
        return None;
    }

    let pose = refine_ransac_pose(&result, triangulator, scratch);

    println!(
        "PnP correspondences: {}, RANSAC inliers: {}, inlier ratio: {:.3}, RANSAC reprojection RMSE: {:?}, selected reprojection RMSE: {:?}",
        scratch.world_points.len(),
        result.inliers.len(),
        result.inliers.len() as f32 / scratch.world_points.len() as f32,
        result.pose.reproj_rmse,
        pose.reproj_rmse,
    );

    Some(pnp_pose_to_isometry(&pose))
}

fn refine_ransac_pose(
    result: &PnPRansacResult,
    triangulator: &StereoTriangulator,
    scratch: &mut OdometryScratch,
) -> PnPResult {
    scratch.inlier_world_points.clear();
    scratch.inlier_image_points.clear();

    for &index in &result.inliers {
        let Some(world_point) = scratch.world_points.get(index).copied() else {
            continue;
        };
        let Some(image_point) = scratch.image_points.get(index).copied() else {
            continue;
        };

        scratch.inlier_world_points.push(world_point);
        scratch.inlier_image_points.push(image_point);
    }

    if scratch.inlier_world_points.len() < MIN_PNP_CORRESPONDENCES {
        return result.pose.clone();
    }

    let refined_pose = match refine_pose_lm(
        &scratch.inlier_world_points,
        &scratch.inlier_image_points,
        triangulator.intrinsics_f32(),
        &result.pose.rotation,
        &result.pose.translation,
        None,
        &LMRefineParams::default(),
    ) {
        Ok(refined_pose) => refined_pose,
        Err(error) => {
            tracing::debug!(
                ?error,
                inliers = result.inliers.len(),
                "PnP LM refinement failed"
            );
            return result.pose.clone();
        }
    };

    if is_refinement_usable(&result.pose, &refined_pose) {
        refined_pose
    } else {
        tracing::debug!(
            ransac_rmse = result.pose.reproj_rmse,
            refined_rmse = refined_pose.reproj_rmse,
            "PnP LM refinement worsened reprojection error"
        );
        result.pose.clone()
    }
}

fn is_refinement_usable(ransac_pose: &PnPResult, refined_pose: &PnPResult) -> bool {
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
    let rotation_array: [f32; 9] = pose.rotation.into();
    let rotation =
        na::Rotation3::from_matrix_unchecked(na::Matrix3::from_column_slice(&rotation_array));
    na::Isometry3::from_parts(
        na::Translation3::new(pose.translation.x, pose.translation.y, pose.translation.z),
        na::UnitQuaternion::from_rotation_matrix(&rotation),
    )
}
