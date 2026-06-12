use color_eyre::eyre::ensure;
use kornia_3d::pnp::{
    PnPMethod, PnPRansacResult, PnPResult, RansacParams, solve_pnp, solve_pnp_ransac,
};
use kornia_algebra::{
    Mat3AF32, SO3F32, Vec2F32, Vec3AF32,
    optim::{HuberLoss, RobustLoss},
};
use nalgebra as na;
use types::parameters::StereoVisualOdometryPoseEstimationParameters;

use crate::{
    feature_extractor::{CurrentLeft, FrameFeatures, KEYPOINTS, Matches, PreviousLeft},
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
struct PoseCorrespondence {
    world_point: Vec3AF32,
    image_point: Vec2F32,
    right_image_point: Option<Vec2F32>,
    weight: f32,
}

#[derive(Clone, Copy)]
struct CorrespondenceResidual {
    image_point: Vec2F32,
    x_offset: f32,
    weight: f32,
}

struct ProjectionResidual {
    residual: na::SVector<f32, 2>,
    projection_x: f32,
    inverse_z: f32,
    inverse_z_squared: f32,
    fx: f32,
    fy: f32,
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
            correspondences: Vec::with_capacity(KEYPOINTS),
            inlier_correspondences: Vec::with_capacity(KEYPOINTS),
            pnp_world_points: Vec::with_capacity(KEYPOINTS),
            pnp_image_points: Vec::with_capacity(KEYPOINTS),
            right_observations_by_left_index: vec![None; KEYPOINTS],
        }
    }
}

pub fn validate_pose_estimation_parameters(
    parameters: &StereoVisualOdometryPoseEstimationParameters,
) -> color_eyre::Result<()> {
    ensure!(
        parameters.minimum_pnp_correspondences >= 4,
        "minimum_pnp_correspondences must be at least 4"
    );
    ensure!(
        parameters.ransac_reprojection_threshold_px.is_finite()
            && parameters.ransac_reprojection_threshold_px > 0.0,
        "ransac_reprojection_threshold_px must be finite and > 0"
    );
    ensure!(
        parameters.ransac_max_iterations > 0,
        "ransac_max_iterations must be > 0"
    );
    ensure!(
        parameters.ransac_confidence.is_finite()
            && parameters.ransac_confidence > 0.0
            && parameters.ransac_confidence < 1.0,
        "ransac_confidence must be finite and in (0, 1)"
    );
    ensure!(
        parameters.lm_max_iterations > 0,
        "lm_max_iterations must be > 0"
    );
    ensure!(
        parameters.lm_initial_lambda.is_finite()
            && parameters.lm_initial_lambda > 0.0
            && parameters.lm_min_lambda.is_finite()
            && parameters.lm_min_lambda > 0.0
            && parameters.lm_max_lambda.is_finite()
            && parameters.lm_max_lambda >= parameters.lm_initial_lambda
            && parameters.lm_initial_lambda >= parameters.lm_min_lambda,
        "LM lambda values must be finite and satisfy lm_min_lambda <= lm_initial_lambda <= lm_max_lambda"
    );
    ensure!(
        parameters.lm_step_tolerance.is_finite() && parameters.lm_step_tolerance > 0.0,
        "lm_step_tolerance must be finite and > 0"
    );
    ensure!(
        parameters.lm_cost_tolerance.is_finite() && parameters.lm_cost_tolerance > 0.0,
        "lm_cost_tolerance must be finite and > 0"
    );
    ensure!(
        parameters.lm_huber_threshold_px.is_finite() && parameters.lm_huber_threshold_px > 0.0,
        "lm_huber_threshold_px must be finite and > 0"
    );
    ensure!(
        parameters.full_weight_disparity_px.is_finite()
            && parameters.full_weight_disparity_px > 0.0,
        "full_weight_disparity_px must be finite and > 0"
    );
    ensure!(
        parameters.min_disparity_weight.is_finite()
            && parameters.min_disparity_weight > 0.0
            && parameters.min_disparity_weight <= 1.0,
        "min_disparity_weight must be finite and in (0, 1]"
    );
    ensure!(
        parameters.max_vertical_disparity_px.is_finite()
            && parameters.max_vertical_disparity_px >= 0.0,
        "max_vertical_disparity_px must be finite and >= 0"
    );

    Ok(())
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
    correspondences: &[PoseCorrespondence],
    intrinsics: &Mat3AF32,
    baseline: f32,
    parameters: &StereoVisualOdometryPoseEstimationParameters,
    initial_pose: &PnPResult,
) -> Result<PnPResult, &'static str> {
    let mut rotation = matrix3_from_mat3a(&initial_pose.rotation);
    let mut translation = vector3_from_vec3a(initial_pose.translation);
    let loss =
        HuberLoss::new(parameters.lm_huber_threshold_px).map_err(|_| "invalid Huber threshold")?;
    let mut cost = total_robust_reprojection_cost(
        correspondences,
        intrinsics,
        baseline,
        &loss,
        &rotation,
        &translation,
    )
    .ok_or("initial reprojection error is invalid")?;
    let mut lambda = parameters.lm_initial_lambda;
    let mut iterations = 0;
    let mut converged = false;

    for iteration in 0..parameters.lm_max_iterations {
        iterations = iteration + 1;
        let (mut normal_matrix, gradient) = build_normal_equations(
            correspondences,
            intrinsics,
            baseline,
            &loss,
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
        if step.norm() < parameters.lm_step_tolerance {
            converged = true;
            break;
        }

        let (candidate_rotation, candidate_translation) =
            apply_pose_step(&rotation, &translation, &step);
        let candidate_cost = total_robust_reprojection_cost(
            correspondences,
            intrinsics,
            baseline,
            &loss,
            &candidate_rotation,
            &candidate_translation,
        )
        .ok_or("candidate reprojection error is invalid")?;

        if candidate_cost < cost {
            let improvement = cost - candidate_cost;
            rotation = candidate_rotation;
            translation = candidate_translation;
            cost = candidate_cost;
            lambda = (lambda * 0.3).max(parameters.lm_min_lambda);

            if improvement < parameters.lm_cost_tolerance {
                converged = true;
                break;
            }
        } else {
            lambda = (lambda * 10.0).min(parameters.lm_max_lambda);
        }
    }

    let rotation = mat3a_from_matrix3(&rotation);
    let translation = vec3a_from_vector3(translation);
    let reproj_rmse = (total_reprojection_error_squared(
        correspondences,
        intrinsics,
        &matrix3_from_mat3a(&rotation),
        &vector3_from_vec3a(translation),
    )
    .ok_or("final reprojection error is invalid")?
        / correspondences.len() as f32)
        .sqrt();

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
    correspondences: &[PoseCorrespondence],
    intrinsics: &Mat3AF32,
    baseline: f32,
    loss: &HuberLoss,
    rotation: &na::Matrix3<f32>,
    translation: &na::Vector3<f32>,
) -> Option<(na::SMatrix<f32, 6, 6>, na::SVector<f32, 6>)> {
    let mut normal_matrix = na::SMatrix::<f32, 6, 6>::zeros();
    let mut gradient = na::SVector::<f32, 6>::zeros();

    for correspondence in correspondences {
        let camera_point = rotation * vector3_from_vec3a(correspondence.world_point) + translation;
        for residual in correspondence_residuals(correspondence, baseline)
            .into_iter()
            .flatten()
        {
            add_residual_block(
                &mut normal_matrix,
                &mut gradient,
                camera_point,
                residual.image_point,
                intrinsics,
                residual.x_offset,
                residual.weight,
                loss,
            )?;
        }
    }

    Some((normal_matrix, gradient))
}

fn add_residual_block(
    normal_matrix: &mut na::SMatrix<f32, 6, 6>,
    gradient: &mut na::SVector<f32, 6>,
    camera_point: na::Vector3<f32>,
    image_point: Vec2F32,
    intrinsics: &Mat3AF32,
    x_offset: f32,
    base_weight: f32,
    loss: &HuberLoss,
) -> Option<()> {
    let (residual, jacobian) =
        residual_and_jacobian_with_x_offset(camera_point, image_point, intrinsics, x_offset)?;
    let scale = weighted_residual_scale(residual.norm_squared(), base_weight, loss)?;
    let weighted_jacobian = jacobian * scale;
    let weighted_residual = residual * scale;

    *normal_matrix += weighted_jacobian.transpose() * weighted_jacobian;
    *gradient += weighted_jacobian.transpose() * weighted_residual;

    Some(())
}

fn residual_and_jacobian_with_x_offset(
    camera_point: na::Vector3<f32>,
    image_point: Vec2F32,
    intrinsics: &Mat3AF32,
    x_offset: f32,
) -> Option<(na::SVector<f32, 2>, na::SMatrix<f32, 2, 6>)> {
    let projection = projection_residual(camera_point, image_point, intrinsics, x_offset)?;

    let du_dx = projection.fx * projection.inverse_z;
    let du_dz = -projection.fx * projection.projection_x * projection.inverse_z_squared;
    let dv_dy = projection.fy * projection.inverse_z;
    let dv_dz = -projection.fy * camera_point.y * projection.inverse_z_squared;

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

    jacobian
        .iter()
        .all(|value| value.is_finite())
        .then_some((projection.residual, jacobian))
}

fn total_robust_reprojection_cost(
    correspondences: &[PoseCorrespondence],
    intrinsics: &Mat3AF32,
    baseline: f32,
    loss: &HuberLoss,
    rotation: &na::Matrix3<f32>,
    translation: &na::Vector3<f32>,
) -> Option<f32> {
    let mut cost = 0.0;

    for correspondence in correspondences {
        let camera_point = rotation * vector3_from_vec3a(correspondence.world_point) + translation;
        for residual in correspondence_residuals(correspondence, baseline)
            .into_iter()
            .flatten()
        {
            cost += residual_block_cost(
                camera_point,
                residual.image_point,
                intrinsics,
                residual.x_offset,
                residual.weight,
                loss,
            )?;
        }
    }

    cost.is_finite().then_some(cost)
}

fn correspondence_residuals(
    correspondence: &PoseCorrespondence,
    baseline: f32,
) -> [Option<CorrespondenceResidual>; 2] {
    [
        Some(CorrespondenceResidual {
            image_point: correspondence.image_point,
            x_offset: 0.0,
            weight: correspondence.weight,
        }),
        correspondence
            .right_image_point
            .map(|image_point| CorrespondenceResidual {
                image_point,
                x_offset: -baseline,
                weight: correspondence.weight,
            }),
    ]
}

fn residual_block_cost(
    camera_point: na::Vector3<f32>,
    image_point: Vec2F32,
    intrinsics: &Mat3AF32,
    x_offset: f32,
    base_weight: f32,
    loss: &HuberLoss,
) -> Option<f32> {
    let residual = residual_with_x_offset(camera_point, image_point, intrinsics, x_offset)?;
    robust_residual_cost(residual.norm_squared(), base_weight, loss)
}

fn residual_with_x_offset(
    camera_point: na::Vector3<f32>,
    image_point: Vec2F32,
    intrinsics: &Mat3AF32,
    x_offset: f32,
) -> Option<na::SVector<f32, 2>> {
    Some(projection_residual(camera_point, image_point, intrinsics, x_offset)?.residual)
}

fn projection_residual(
    camera_point: na::Vector3<f32>,
    image_point: Vec2F32,
    intrinsics: &Mat3AF32,
    x_offset: f32,
) -> Option<ProjectionResidual> {
    if !camera_point.iter().all(|value| value.is_finite()) || camera_point.z <= 0.0 {
        return None;
    }

    let fx = intrinsics.x_axis().x;
    let fy = intrinsics.y_axis().y;
    let cx = intrinsics.z_axis().x;
    let cy = intrinsics.z_axis().y;
    let inverse_z = 1.0 / camera_point.z;
    let projection_x = camera_point.x + x_offset;
    if !projection_x.is_finite() || !x_offset.is_finite() {
        return None;
    }

    let projected_x = fx * projection_x * inverse_z + cx;
    let projected_y = fy * camera_point.y * inverse_z + cy;
    let residual =
        na::SVector::<f32, 2>::new(projected_x - image_point.x, projected_y - image_point.y);

    residual
        .iter()
        .all(|value| value.is_finite())
        .then_some(ProjectionResidual {
            residual,
            projection_x,
            inverse_z,
            inverse_z_squared: inverse_z * inverse_z,
            fx,
            fy,
        })
}

fn robust_residual_cost(
    residual_norm_squared: f32,
    base_weight: f32,
    loss: &HuberLoss,
) -> Option<f32> {
    if !residual_norm_squared.is_finite() || !base_weight.is_finite() || base_weight <= 0.0 {
        return None;
    }

    let cost = base_weight * loss.rho(residual_norm_squared);

    cost.is_finite().then_some(cost)
}

fn weighted_residual_scale(
    residual_norm_squared: f32,
    base_weight: f32,
    loss: &HuberLoss,
) -> Option<f32> {
    if !residual_norm_squared.is_finite() || !base_weight.is_finite() || base_weight <= 0.0 {
        return None;
    }

    let weight = base_weight * loss.weight(residual_norm_squared);

    (weight.is_finite() && weight > 0.0).then_some(weight.sqrt())
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

fn total_reprojection_error_squared(
    correspondences: &[PoseCorrespondence],
    intrinsics: &Mat3AF32,
    rotation: &na::Matrix3<f32>,
    translation: &na::Vector3<f32>,
) -> Option<f32> {
    let mut error = 0.0;

    for correspondence in correspondences {
        let camera_point = rotation * vector3_from_vec3a(correspondence.world_point) + translation;
        let residual =
            residual_with_x_offset(camera_point, correspondence.image_point, intrinsics, 0.0)?;
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
