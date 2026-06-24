use kornia_3d::pnp::PnPResult;
use kornia_algebra::{
    Mat3AF32, SO3F32, Vec2F32, Vec3AF32,
    optim::{HuberLoss, RobustLoss},
};
use nalgebra as na;

use crate::{
    odometry::PoseCorrespondence, parameters::StereoVisualOdometryPoseEstimationParameters,
};

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

pub fn refine_pose_lm_direct(
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
                intrinsics,
                residual,
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
    intrinsics: &Mat3AF32,
    correspondence_residual: CorrespondenceResidual,
    loss: &HuberLoss,
) -> Option<()> {
    let (residual, jacobian) = residual_and_jacobian_with_x_offset(
        camera_point,
        correspondence_residual.image_point,
        intrinsics,
        correspondence_residual.x_offset,
    )?;
    let scale = weighted_residual_scale(
        residual.norm_squared(),
        correspondence_residual.weight,
        loss,
    )?;
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

pub fn residual_with_x_offset(
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

pub fn matrix3_from_mat3a(matrix: &Mat3AF32) -> na::Matrix3<f32> {
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

pub fn vector3_from_vec3a(vector: Vec3AF32) -> na::Vector3<f32> {
    na::Vector3::new(vector.x, vector.y, vector.z)
}

fn vec3a_from_vector3(vector: na::Vector3<f32>) -> Vec3AF32 {
    Vec3AF32::new(vector.x, vector.y, vector.z)
}
