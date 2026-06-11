use kornia_3d::pnp::{
    PnPMethod, PnPRansacResult, PnPResult, RansacParams, solve_pnp, solve_pnp_ransac,
};
use kornia_algebra::{Mat3AF32, SO3F32, Vec2F32, Vec3AF32};
use nalgebra as na;

use crate::{
    feature_extractor::{CurrentLeft, FrameFeatures, KEYPOINTS, Matches, PreviousLeft},
    triangulator::{StereoPoint, StereoTriangulator},
};

const MIN_PNP_CORRESPONDENCES: usize = 8;
const RANSAC_REPROJ_THRESHOLD_PX: f32 = 6.0;
const LM_MAX_ITERATIONS: usize = 20;
const LM_INITIAL_LAMBDA: f32 = 1e-3;
const LM_MIN_LAMBDA: f32 = 1e-7;
const LM_MAX_LAMBDA: f32 = 1e10;
const LM_STEP_TOLERANCE: f32 = 1e-6;
const LM_COST_TOLERANCE: f32 = 1e-6;

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

    let pose = if let Some(pose) = estimate_outlier_free_pose(triangulator, scratch) {
        pose
    } else {
        let params = RansacParams {
            max_iterations: 100,
            reproj_threshold_px: RANSAC_REPROJ_THRESHOLD_PX,
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

        refine_ransac_pose(&result, triangulator, scratch)
    };

    Some(pnp_pose_to_isometry(&pose))
}

fn estimate_outlier_free_pose(
    triangulator: &StereoTriangulator,
    scratch: &OdometryScratch,
) -> Option<PnPResult> {
    let pose = match solve_pnp(
        &scratch.world_points,
        &scratch.image_points,
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
        .is_some_and(|rmse| rmse.is_finite() && rmse <= RANSAC_REPROJ_THRESHOLD_PX)
    {
        return None;
    }

    let refined_pose = refine_pose(
        &pose,
        &scratch.world_points,
        &scratch.image_points,
        triangulator,
    );

    all_reprojection_errors_within_threshold(&refined_pose, triangulator.intrinsics_f32(), scratch)
        .then_some(refined_pose)
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

    refine_pose(
        &result.pose,
        &scratch.inlier_world_points,
        &scratch.inlier_image_points,
        triangulator,
    )
}

fn refine_pose(
    initial_pose: &PnPResult,
    world_points: &[Vec3AF32],
    image_points: &[Vec2F32],
    triangulator: &StereoTriangulator,
) -> PnPResult {
    if world_points.len() < MIN_PNP_CORRESPONDENCES {
        return initial_pose.clone();
    }

    let refined_pose = match refine_pose_lm_direct(
        world_points,
        image_points,
        triangulator.intrinsics_f32(),
        initial_pose,
    ) {
        Ok(refined_pose) => refined_pose,
        Err(error) => {
            tracing::debug!(
                error,
                correspondences = world_points.len(),
                "PnP LM refinement failed"
            );
            return initial_pose.clone();
        }
    };

    if is_refinement_usable(initial_pose, &refined_pose) {
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

fn refine_pose_lm_direct(
    world_points: &[Vec3AF32],
    image_points: &[Vec2F32],
    intrinsics: &Mat3AF32,
    initial_pose: &PnPResult,
) -> Result<PnPResult, &'static str> {
    let mut rotation = matrix3_from_mat3a(&initial_pose.rotation);
    let mut translation = vector3_from_vec3a(initial_pose.translation);
    let mut cost = total_reprojection_error_squared(
        world_points,
        image_points,
        intrinsics,
        &rotation,
        &translation,
    )
    .ok_or("initial reprojection error is invalid")?;
    let mut lambda = LM_INITIAL_LAMBDA;
    let mut iterations = 0;
    let mut converged = false;

    for iteration in 0..LM_MAX_ITERATIONS {
        iterations = iteration + 1;
        let (mut normal_matrix, gradient) = build_normal_equations(
            world_points,
            image_points,
            intrinsics,
            &rotation,
            &translation,
        )
        .ok_or("normal equations contain invalid residuals")?;
        for index in 0..6 {
            normal_matrix[(index, index)] += lambda;
        }

        let step = normal_matrix
            .lu()
            .solve(&(-gradient))
            .ok_or("normal equations are singular")?;
        if step.norm() < LM_STEP_TOLERANCE {
            converged = true;
            break;
        }

        let (candidate_rotation, candidate_translation) =
            apply_pose_step(&rotation, &translation, &step);
        let candidate_cost = total_reprojection_error_squared(
            world_points,
            image_points,
            intrinsics,
            &candidate_rotation,
            &candidate_translation,
        )
        .ok_or("candidate reprojection error is invalid")?;

        if candidate_cost < cost {
            let improvement = cost - candidate_cost;
            rotation = candidate_rotation;
            translation = candidate_translation;
            cost = candidate_cost;
            lambda = (lambda * 0.3).max(LM_MIN_LAMBDA);

            if improvement < LM_COST_TOLERANCE {
                converged = true;
                break;
            }
        } else {
            lambda = (lambda * 10.0).min(LM_MAX_LAMBDA);
        }
    }

    let rotation = mat3a_from_matrix3(&rotation);
    let translation = vec3a_from_vector3(translation);
    let reproj_rmse = (cost / world_points.len() as f32).sqrt();

    Ok(PnPResult {
        rotation,
        translation,
        rvec: SO3F32::from_matrix(&rotation).log(),
        reproj_rmse: Some(reproj_rmse),
        num_iterations: Some(iterations),
        converged: Some(converged),
    })
}

fn build_normal_equations(
    world_points: &[Vec3AF32],
    image_points: &[Vec2F32],
    intrinsics: &Mat3AF32,
    rotation: &na::Matrix3<f32>,
    translation: &na::Vector3<f32>,
) -> Option<(na::SMatrix<f32, 6, 6>, na::SVector<f32, 6>)> {
    let mut normal_matrix = na::SMatrix::<f32, 6, 6>::zeros();
    let mut gradient = na::SVector::<f32, 6>::zeros();

    for (&world_point, &image_point) in world_points.iter().zip(image_points.iter()) {
        let camera_point = rotation * vector3_from_vec3a(world_point) + translation;
        let (residual, jacobian) = residual_and_jacobian(camera_point, image_point, intrinsics)?;

        normal_matrix += jacobian.transpose() * jacobian;
        gradient += jacobian.transpose() * residual;
    }

    Some((normal_matrix, gradient))
}

fn residual_and_jacobian(
    camera_point: na::Vector3<f32>,
    image_point: Vec2F32,
    intrinsics: &Mat3AF32,
) -> Option<(na::SVector<f32, 2>, na::SMatrix<f32, 2, 6>)> {
    if !camera_point.z.is_finite() || camera_point.z <= 0.0 {
        return None;
    }

    let fx = intrinsics.x_axis().x;
    let fy = intrinsics.y_axis().y;
    let cx = intrinsics.z_axis().x;
    let cy = intrinsics.z_axis().y;
    let inverse_z = 1.0 / camera_point.z;
    let inverse_z_squared = inverse_z * inverse_z;

    let projected_x = fx * camera_point.x * inverse_z + cx;
    let projected_y = fy * camera_point.y * inverse_z + cy;
    let residual =
        na::SVector::<f32, 2>::new(projected_x - image_point.x, projected_y - image_point.y);

    let du_dx = fx * inverse_z;
    let du_dz = -fx * camera_point.x * inverse_z_squared;
    let dv_dy = fy * inverse_z;
    let dv_dz = -fy * camera_point.y * inverse_z_squared;

    let jacobian = na::SMatrix::<f32, 2, 6>::from_row_slice(&[
        du_dx,
        0.0,
        du_dz,
        du_dz * camera_point.y,
        du_dx * camera_point.z - du_dz * camera_point.x,
        -du_dx * camera_point.y,
        0.0,
        dv_dy,
        dv_dz,
        -dv_dy * camera_point.z + dv_dz * camera_point.y,
        -dv_dz * camera_point.x,
        dv_dy * camera_point.x,
    ]);

    (residual.iter().all(|value| value.is_finite())
        && jacobian.iter().all(|value| value.is_finite()))
    .then_some((residual, jacobian))
}

fn total_reprojection_error_squared(
    world_points: &[Vec3AF32],
    image_points: &[Vec2F32],
    intrinsics: &Mat3AF32,
    rotation: &na::Matrix3<f32>,
    translation: &na::Vector3<f32>,
) -> Option<f32> {
    let mut error = 0.0;

    for (&world_point, &image_point) in world_points.iter().zip(image_points.iter()) {
        let camera_point = rotation * vector3_from_vec3a(world_point) + translation;
        let (residual, _) = residual_and_jacobian(camera_point, image_point, intrinsics)?;
        error += residual.norm_squared();
    }

    error.is_finite().then_some(error)
}

fn apply_pose_step(
    rotation: &na::Matrix3<f32>,
    translation: &na::Vector3<f32>,
    step: &na::SVector<f32, 6>,
) -> (na::Matrix3<f32>, na::Vector3<f32>) {
    let translation_step = na::Vector3::new(step[0], step[1], step[2]);
    let rotation_step =
        na::UnitQuaternion::from_scaled_axis(na::Vector3::new(step[3], step[4], step[5]));
    let rotation_step_matrix = rotation_step.to_rotation_matrix();

    (
        rotation_step_matrix.matrix() * rotation,
        rotation_step_matrix * translation + translation_step,
    )
}

fn all_reprojection_errors_within_threshold(
    pose: &PnPResult,
    intrinsics: &Mat3AF32,
    scratch: &OdometryScratch,
) -> bool {
    let threshold_squared = RANSAC_REPROJ_THRESHOLD_PX * RANSAC_REPROJ_THRESHOLD_PX;

    scratch
        .world_points
        .iter()
        .zip(scratch.image_points.iter())
        .all(|(&world_point, &image_point)| {
            reprojection_squared_error(world_point, image_point, pose, intrinsics)
                .is_some_and(|error| error <= threshold_squared)
        })
}

fn reprojection_squared_error(
    world_point: Vec3AF32,
    image_point: Vec2F32,
    pose: &PnPResult,
    intrinsics: &Mat3AF32,
) -> Option<f32> {
    let camera_point = pose.rotation * world_point + pose.translation;
    if !camera_point.z.is_finite() || camera_point.z <= 0.0 {
        return None;
    }

    let fx = intrinsics.x_axis().x;
    let fy = intrinsics.y_axis().y;
    let cx = intrinsics.z_axis().x;
    let cy = intrinsics.z_axis().y;
    let projected_x = fx * camera_point.x / camera_point.z + cx;
    let projected_y = fy * camera_point.y / camera_point.z + cy;
    let error_x = projected_x - image_point.x;
    let error_y = projected_y - image_point.y;
    let error = error_x * error_x + error_y * error_y;

    error.is_finite().then_some(error)
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

fn matrix3_from_mat3a(matrix: &Mat3AF32) -> na::Matrix3<f32> {
    let array: [f32; 9] = (*matrix).into();
    na::Matrix3::from_column_slice(&array)
}

fn mat3a_from_matrix3(matrix: &na::Matrix3<f32>) -> Mat3AF32 {
    Mat3AF32::from_cols(
        Vec3AF32::new(matrix[(0, 0)], matrix[(1, 0)], matrix[(2, 0)]),
        Vec3AF32::new(matrix[(0, 1)], matrix[(1, 1)], matrix[(2, 1)]),
        Vec3AF32::new(matrix[(0, 2)], matrix[(1, 2)], matrix[(2, 2)]),
    )
}

fn vector3_from_vec3a(vector: Vec3AF32) -> na::Vector3<f32> {
    na::Vector3::new(vector.x, vector.y, vector.z)
}

fn vec3a_from_vector3(vector: na::Vector3<f32>) -> Vec3AF32 {
    Vec3AF32::new(vector.x, vector.y, vector.z)
}

fn pnp_pose_to_isometry(pose: &PnPResult) -> na::Isometry3<f32> {
    let rotation = na::Rotation3::from_matrix_unchecked(matrix3_from_mat3a(&pose.rotation));
    na::Isometry3::from_parts(
        na::Translation3::new(pose.translation.x, pose.translation.y, pose.translation.z),
        na::UnitQuaternion::from_rotation_matrix(&rotation),
    )
}
