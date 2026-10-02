use coordinate_systems::{Field, Pixel, Robot};
use linear_algebra::{Isometry3, Point2, Point3};
use linear_sum_assignment::{AssignmentSolver, Objective};
use nalgebra::{
    Matrix2, Matrix2x3, Matrix2x6, Matrix3, Matrix3x6, Matrix6, SMatrix, SVector, Vector3,
};
use ndarray::Array2;
use types::{localization::PoseEstimate, visual_localization::FieldMarkAssociation};

use crate::{
    AssociationResult, DetectedVisualFeature, DetectedVisualFeatures,
    FieldMarkAssociationParameters, TrackingAssociationInput as AssociationInput,
    VisualFeatureClass, features::raw_detections, map::LandmarkMap,
};

#[derive(Clone, Copy)]
struct Prediction {
    pixel: Point2<Pixel>,
    covariance: Matrix2<f64>,
    jacobian: Matrix2x6<f64>,
}

struct PredictionNoise {
    pose: Matrix6<f64>,
    pixel_variance: f64,
}

pub(crate) fn associate(
    input: AssociationInput<'_>,
    parameters: &FieldMarkAssociationParameters,
) -> Option<AssociationResult> {
    let age = validated_age(input, parameters)?;
    let map = LandmarkMap::new(
        input.field_dimensions,
        parameters.global_localizer.symmetry_epsilon,
    );
    if map
        .landmarks
        .iter()
        .any(|landmark| !landmark.xy.coords().inner.iter().all(|x| x.is_finite()))
    {
        return None;
    }
    let config = parameters.global_localizer;
    let detections = filter_detections(input.visual_features, &map, config)?;
    if detections.len() < config.min_inliers {
        return None;
    }
    let estimate = input.geometry.estimate;
    let noise = PredictionNoise {
        pose: estimate.covariance + process_covariance(estimate.pose, age, parameters),
        pixel_variance: f64::from(config.detection_pixel_sigma).powi(2),
    };
    let predictions = map
        .landmarks
        .iter()
        .map(|point| project_landmark(point.xy.extend(0.0), input, &noise))
        .collect::<Vec<_>>();
    match_predictions(&detections, &map, &predictions, &noise, parameters)
}

fn validated_age(
    input: AssociationInput<'_>,
    parameters: &FieldMarkAssociationParameters,
) -> Option<f32> {
    let tracking = parameters.tracking;
    let estimate = input.geometry.estimate;
    let last_successful_solve = input.geometry.last_successful_solve;
    if !valid_geometry(input, estimate, parameters.global_localizer)
        || !valid_covariance(estimate.covariance, tracking)
    {
        return None;
    }
    if input.time < last_successful_solve {
        return None;
    }
    let age = input.time.duration_since(last_successful_solve);
    if age > tracking.max_age {
        return None;
    }
    Some(age.as_secs_f32())
}

fn valid_geometry(
    input: AssociationInput<'_>,
    estimate: PoseEstimate<Robot, Field>,
    config: crate::GlobalLocalizerParameters,
) -> bool {
    let intrinsic = input.camera_intrinsic;
    [
        estimate.pose.inner.to_homogeneous(),
        input.robot_to_camera.inner.to_homogeneous().cast::<f64>(),
    ]
    .iter()
    .all(|matrix| matrix.iter().all(|x| x.is_finite()))
        && intrinsic.is_valid()
        && input.visual_features.supported_feature_count() <= config.max_input_detections
}

fn valid_covariance(
    covariance: Matrix6<f64>,
    parameters: crate::TrackingAssociationParameters,
) -> bool {
    if !covariance.iter().all(|x| x.is_finite())
        || (covariance - covariance.transpose()).amax()
            > f64::from(parameters.covariance_symmetry_tolerance)
    {
        return false;
    }
    covariance
        .try_symmetric_eigen(f64::EPSILON, parameters.covariance_eigen_max_iterations)
        .is_some_and(|eigen| {
            eigen.eigenvalues.iter().all(|value| {
                value.is_finite() && *value >= -f64::from(parameters.covariance_psd_tolerance)
            })
        })
}

fn process_covariance(
    robot_to_field: Isometry3<Robot, Field, f64>,
    age: f32,
    parameters: &FieldMarkAssociationParameters,
) -> Matrix6<f64> {
    let config = parameters.global_localizer;
    let tracking = parameters.tracking;
    // Horizontal isotropy makes these floors invariant to the Local-to-Field yaw.
    let field_to_robot = robot_to_field.inner.rotation.inverse().to_rotation_matrix();
    let rotation_noise = Matrix3::from_diagonal(&Vector3::new(
        f64::from(config.imu_tilt_sigma).powi(2),
        f64::from(config.imu_tilt_sigma).powi(2),
        (f64::from(age) * f64::from(tracking.yaw_sigma_per_second)).powi(2),
    ));
    let position_variance =
        (f64::from(age) * f64::from(tracking.position_sigma_per_second)).powi(2);
    let translation_noise = Matrix3::from_diagonal(&Vector3::new(
        position_variance,
        position_variance,
        position_variance + f64::from(config.height_sigma).powi(2),
    ));
    let mut current_covariance = Matrix6::zeros();
    current_covariance.fixed_view_mut::<3, 3>(0, 0).copy_from(
        &(field_to_robot.matrix() * rotation_noise * field_to_robot.matrix().transpose()),
    );
    current_covariance.fixed_view_mut::<3, 3>(3, 3).copy_from(
        &(field_to_robot.matrix() * translation_noise * field_to_robot.matrix().transpose()),
    );
    current_covariance
}

fn filter_detections(
    features: &DetectedVisualFeatures,
    map: &LandmarkMap,
    config: crate::GlobalLocalizerParameters,
) -> Option<Vec<(VisualFeatureClass, DetectedVisualFeature)>> {
    let mut detections = raw_detections(features)
        .filter(|(_, feature)| {
            (config.confidence_threshold..=1.0).contains(&feature.confidence)
                && feature.pixel.coords().inner.iter().all(|x| x.is_finite())
        })
        .collect::<Vec<_>>();
    detections.sort_by(|(a, x), (b, y)| {
        (y.confidence * map.rarity_weight(*b)).total_cmp(&(x.confidence * map.rarity_weight(*a)))
    });
    let mut retained: Vec<(VisualFeatureClass, DetectedVisualFeature)> = Vec::new();
    for (class, detection) in detections {
        if retained.iter().any(|(other_class, other)| {
            *other_class == class
                && (other.pixel - detection.pixel).inner.norm() <= config.duplicate_pixel_distance
        }) {
            continue;
        }
        if retained.len() == config.max_retained_detections {
            return None;
        }
        retained.push((class, detection));
    }
    Some(retained)
}

fn match_predictions(
    detections: &[(VisualFeatureClass, DetectedVisualFeature)],
    map: &LandmarkMap,
    predictions: &[Option<Prediction>],
    noise: &PredictionNoise,
    parameters: &FieldMarkAssociationParameters,
) -> Option<AssociationResult> {
    let log_likelihoods = prediction_log_likelihoods(
        detections,
        map,
        predictions,
        parameters.global_localizer.mahalanobis_gate,
    )?;
    let pairs = match detections.len() {
        3 => joint_assignment::<6>(detections, predictions, &log_likelihoods, noise, parameters),
        4 => joint_assignment::<8>(detections, predictions, &log_likelihoods, noise, parameters),
        5 => joint_assignment::<10>(detections, predictions, &log_likelihoods, noise, parameters),
        _ => {
            // ponytail: more than five features retain marginal assignment, not a joint likelihood.
            // Extend joint search only with a measured real-time bound. Row normalization bounds weights.
            let mut benefits = log_likelihoods;
            let m = map.landmarks.len();
            for (row, (_, detection)) in detections.iter().enumerate() {
                let maximum = benefits
                    .row(row)
                    .iter()
                    .take(m)
                    .copied()
                    .fold(f32::NEG_INFINITY, f32::max);
                for column in 0..benefits.ncols() {
                    let likelihood = benefits[(row, column)];
                    benefits[(row, column)] = if column < m && likelihood.is_finite() {
                        detection.confidence * (likelihood - maximum).exp()
                    } else {
                        0.0
                    };
                }
            }
            unique_assignment(
                &mut benefits,
                map.landmarks.len(),
                parameters.tracking.score_ratio,
            )
        }
    }?;
    certify(detections, map, predictions, &pairs, parameters)
}

fn prediction_log_likelihoods(
    detections: &[(VisualFeatureClass, DetectedVisualFeature)],
    map: &LandmarkMap,
    predictions: &[Option<Prediction>],
    mahalanobis_gate: f32,
) -> Option<Array2<f32>> {
    let n = detections.len();
    let m = map.landmarks.len();
    // The extra n columns become zero-benefit unassignment slots after normalization.
    let mut log_likelihoods = Array2::from_elem((n, m + n), f32::NEG_INFINITY);
    for (landmark, prediction) in predictions.iter().enumerate() {
        let Some(prediction) = prediction else {
            continue;
        };
        let cholesky = prediction.covariance.cholesky()?;
        let log_determinant = 2.0 * cholesky.l().diagonal().iter().map(|x| x.ln()).sum::<f64>();
        let information = cholesky.inverse();
        for (row, (_, detection)) in detections
            .iter()
            .enumerate()
            .filter(|(_, (class, _))| *class == map.landmarks[landmark].class)
        {
            let error = (prediction.pixel - detection.pixel).inner.cast::<f64>();
            let mahalanobis = error.dot(&(information * error));
            // The hard output distance must not hide a statistically plausible rival.
            if (0.0..f64::from(mahalanobis_gate)).contains(&mahalanobis) {
                log_likelihoods[(row, landmark)] = (-0.5 * (mahalanobis + log_determinant)) as f32;
            }
        }
    }
    Some(log_likelihoods)
}

fn joint_assignment<const D: usize>(
    detections: &[(VisualFeatureClass, DetectedVisualFeature)],
    predictions: &[Option<Prediction>],
    gated: &Array2<f32>,
    noise: &PredictionNoise,
    parameters: &FieldMarkAssociationParameters,
) -> Option<Vec<(usize, usize)>> {
    let n = detections.len();
    debug_assert_eq!(D, 2 * n);
    if detections
        .iter()
        .any(|(_, detection)| detection.confidence == 0.0)
    {
        return None;
    }
    let candidates = (0..n)
        .map(|row| {
            (0..predictions.len())
                .filter(|&column| gated[(row, column)].is_finite())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let mut best = None;
    let mut best_score = f64::NEG_INFINITY;
    let mut rival_score = f64::NEG_INFINITY;
    let mut best_error = f64::INFINITY;
    let mut remaining_work = parameters.global_localizer.max_work;
    let mut columns = [0; 5];
    let mut next_candidate_index = [0; 5];
    let mut depth = 0;
    // Only columns[..depth] and its covariance prefix are valid; deeper entries
    // are stale until overwritten. Each depth retains its next candidate cursor.
    let mut covariance = SMatrix::<f64, D, D>::zeros();
    let mut error = SVector::<f64, D>::zeros();
    // Iterative depth-first enumeration. Charge every attempted extension, even duplicate IDs;
    // exhaustion after a candidate still rejects. Each complete assignment needs one <=10D solve.
    loop {
        if next_candidate_index[depth] == candidates[depth].len() {
            if depth == 0 {
                break;
            }
            next_candidate_index[depth] = 0;
            depth -= 1;
            continue;
        }
        remaining_work = remaining_work.checked_sub(1)?;
        let column = candidates[depth][next_candidate_index[depth]];
        next_candidate_index[depth] += 1;
        if columns[..depth].contains(&column) {
            continue;
        }
        columns[depth] = column;
        let prediction = predictions[column]?;
        let projected_covariance = prediction.jacobian * noise.pose;
        // Reuse the prefix covariance when extending/backtracking. Only the new row and column
        // change: C_ij = J_i (P + Q) J_j^T, with the pixel floor on diagonal blocks.
        // This is covariance caching, not pruning; every admitted distinct rival is still scored.
        for (row, &other) in columns[..depth].iter().enumerate() {
            let other = predictions[other]?;
            let block = projected_covariance * other.jacobian.transpose();
            covariance
                .fixed_view_mut::<2, 2>(2 * depth, 2 * row)
                .copy_from(&block);
            covariance
                .fixed_view_mut::<2, 2>(2 * row, 2 * depth)
                .copy_from(&block.transpose());
        }
        covariance
            .fixed_view_mut::<2, 2>(2 * depth, 2 * depth)
            .copy_from(&prediction.covariance);
        error.fixed_rows_mut::<2>(2 * depth).copy_from(
            &(prediction.pixel - detections[depth].1.pixel)
                .inner
                .cast::<f64>(),
        );
        if depth + 1 < n {
            depth += 1;
            continue;
        }
        // f64 preserves the pixel floor when large shared uncertainties nearly cancel.
        // A numerical failure cannot silently remove an unscored rival.
        let cholesky = covariance.cholesky()?;
        let mahalanobis = error.dot(&cholesky.solve(&error));
        let log_determinant = 2.0 * cholesky.l().diagonal().iter().map(|x| x.ln()).sum::<f64>();
        let score = -0.5 * (mahalanobis + log_determinant);
        if !score.is_finite() || mahalanobis < 0.0 {
            return None;
        }
        // Confidence and the Gaussian normalizer are constant across complete assignments.
        if score > best_score {
            rival_score = best_score;
            best_score = score;
            best_error = mahalanobis;
            best = Some(columns);
        } else {
            rival_score = rival_score.max(score);
        }
    }
    let best = best?;
    // Keep the marginal gates, but test the full 2nD residual at the same tail probability
    // as the configured 2D gate, without changing scores or hiding rivals.
    let joint_gate = joint_mahalanobis_gate(parameters.global_localizer.mahalanobis_gate, n);
    if best_error >= joint_gate
        || best_score - rival_score <= f64::from(parameters.tracking.score_ratio).ln()
    {
        return None;
    }
    Some(best[..n].iter().copied().enumerate().collect())
}

fn joint_mahalanobis_gate(marginal_gate: f32, features: usize) -> f64 {
    let gate = f64::from(marginal_gate);
    debug_assert!((1..=5).contains(&features));
    if features == 1 {
        return gate;
    }
    // P(chi^2_2 > g) = exp(-g/2). For 2n degrees of freedom, the log survival is
    // -x/2 + ln(sum_{k=0}^{n-1} (x/2)^k/k!). No parameters are fitted by association.
    let log_tail = |x: f64| {
        let half = x / 2.0;
        if half < 1.0 {
            // The survival formula cancels near zero. Instead compute the small CDF as
            // exp(-x/2) * sum_{k=n}^{infinity} (x/2)^k/k!, then use log1p(-CDF).
            let mut term = 1.0;
            for k in 1..=features {
                term *= half / k as f64;
            }
            let mut sum = term;
            for k in features + 1..=features + 32 {
                term *= half / k as f64;
                sum += term;
            }
            (-(sum * (-half).exp())).ln_1p()
        } else {
            let mut term = 1.0;
            let mut sum = term;
            for k in 1..features {
                term *= half / k as f64;
                sum += term;
            }
            sum.ln() - half
        }
    };
    let mut lower = gate;
    // Laurent-Massart: P(chi^2_2n > 2n + 2*sqrt(n*g) + g) <= exp(-g/2).
    // This is only a finite bisection bracket, not a scaled acceptance threshold.
    let mut upper = gate + 2.0 * (features as f64 * gate).sqrt() + 2.0 * features as f64;
    for _ in 0..128 {
        let midpoint = lower + (upper - lower) / 2.0;
        if midpoint == lower || midpoint == upper {
            break;
        }
        if log_tail(midpoint) > -gate / 2.0 {
            lower = midpoint;
        } else {
            upper = midpoint;
        }
    }
    // Choose the conservative side of the final floating-point bracket.
    lower
}

fn certify(
    detections: &[(VisualFeatureClass, DetectedVisualFeature)],
    map: &LandmarkMap,
    predictions: &[Option<Prediction>],
    pairs: &[(usize, usize)],
    parameters: &FieldMarkAssociationParameters,
) -> Option<AssociationResult> {
    let config = parameters.global_localizer;
    let tracking = parameters.tracking;
    if pairs.len() < config.min_inliers
        || pairs.iter().any(|&(row, column)| {
            predictions[column].is_none_or(|prediction| {
                (prediction.pixel - detections[row].1.pixel)
                    .inner
                    .norm_squared()
                    > tracking.max_pixel_distance.powi(2)
            })
        })
    {
        return None;
    }
    Some(AssociationResult {
        associations: pairs
            .iter()
            .map(|&(row, column)| FieldMarkAssociation {
                detection: detections[row].1.pixel,
                field_point: map.landmarks[column].xy.extend(0.0),
            })
            .collect(),
        source: types::visual_localization::VisualAssociationSource::Tracking,
        // GlobalLocalizationDebug has a metric residual, not an image-space residual.
        debug: None,
    })
}

fn project_landmark(
    point: Point3<Field>,
    input: AssociationInput<'_>,
    noise: &PredictionNoise,
) -> Option<Prediction> {
    let point_robot = Isometry3::<Robot, Field>::wrap(input.geometry.estimate.pose.inner.cast())
        .inverse()
        * point;
    let camera = input.robot_to_camera * point_robot;
    if camera.z() <= 0.0 || !camera.coords().inner.iter().all(|x| x.is_finite()) {
        return None;
    }
    let intrinsic = input.camera_intrinsic;
    let pixel = intrinsic.project(camera.coords());
    let projection = Matrix2x3::new(
        intrinsic.focals.x / camera.z(),
        0.0,
        -intrinsic.focals.x * camera.x() / camera.z().powi(2),
        0.0,
        intrinsic.focals.y / camera.z(),
        -intrinsic.focals.y * camera.y() / camera.z().powi(2),
    );
    let inverse_point = |p: Vector3<f32>| {
        Matrix3x6::from_columns(&[
            p.cross(&Vector3::x()),
            p.cross(&Vector3::y()),
            p.cross(&Vector3::z()),
            -Vector3::x(),
            -Vector3::y(),
            -Vector3::z(),
        ])
    };
    // Right-local pose perturbation: J_camera = R_rc [skew(point_robot), -I].
    let jacobian = projection
        * input
            .robot_to_camera
            .inner
            .rotation
            .to_rotation_matrix()
            .matrix()
        * inverse_point(point_robot.coords().inner);
    let jacobian = jacobian.cast::<f64>();
    let covariance =
        jacobian * noise.pose * jacobian.transpose() + Matrix2::identity() * noise.pixel_variance;
    (pixel.coords().inner.iter().all(|x| x.is_finite()) && covariance.iter().all(|x| x.is_finite()))
        .then_some(Prediction {
            pixel,
            covariance,
            jacobian,
        })
}

fn unique_assignment(
    benefits: &mut Array2<f32>,
    landmarks: usize,
    score_ratio: f32,
) -> Option<Vec<(usize, usize)>> {
    let mut assignment = AssignmentSolver::new(benefits.dim());
    let columns = assignment
        .solve(benefits.view(), Objective::Maximize)
        .ok()?;
    let pairs = columns
        .iter()
        .enumerate()
        .filter_map(|(row, column)| {
            let column = (*column)?;
            (column < landmarks && benefits[(row, column)] > 0.0).then_some((row, column))
        })
        .collect::<Vec<_>>();
    let score = pairs.iter().map(|&(r, c)| benefits[(r, c)]).sum::<f32>();
    // Every distinct assignment omits at least one winning edge. This checks all alternatives
    // with one additional bounded assignment solve per winning edge, without fitting candidate poses.
    for &(row, column) in &pairs {
        let saved = benefits[(row, column)];
        // With one zero-benefit dummy per row, zeroing this edge has the same
        // optimal score as forbidding it: its row can always use a free dummy.
        benefits[(row, column)] = 0.0;
        let alternative = assignment
            .solve(benefits.view(), Objective::Maximize)
            .ok()?;
        let alternative_score = alternative
            .iter()
            .enumerate()
            .filter_map(|(r, c)| c.map(|c| benefits[(r, c)]))
            .sum::<f32>();
        benefits[(row, column)] = saved;
        if score - alternative_score <= saved * (1.0 - 1.0 / score_ratio) {
            return None;
        }
    }
    Some(pairs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn covariance_numerical_tolerances_are_applied() {
        let mut parameters = crate::TrackingAssociationParameters::default();
        let mut covariance = Matrix6::identity();
        covariance[(0, 1)] = 5.0e-5;
        assert!(!valid_covariance(covariance, parameters));
        parameters.covariance_symmetry_tolerance = 1.0e-4;
        assert!(valid_covariance(covariance, parameters));
        covariance = Matrix6::identity();
        covariance[(0, 0)] = -5.0e-6;
        assert!(!valid_covariance(covariance, parameters));
        parameters.covariance_psd_tolerance = 1.0e-5;
        assert!(valid_covariance(covariance, parameters));
    }

    #[test]
    fn retained_detection_limit_can_exceed_the_old_ceiling() {
        let map = LandmarkMap::new(
            &types::field_dimensions::FieldDimensions::SPL_2025,
            crate::GlobalLocalizerParameters::default().symmetry_epsilon,
        );
        let features = DetectedVisualFeatures {
            penalty_spots: (0..33)
                .map(|index| DetectedVisualFeature {
                    pixel: linear_algebra::point![index as f32 * 10.0, 100.0],
                    confidence: 1.0,
                })
                .collect(),
            ..Default::default()
        };
        let mut config = crate::GlobalLocalizerParameters::default();
        assert!(filter_detections(&features, &map, config).is_none());
        config.max_retained_detections = 33;
        config.validate().unwrap();
        assert_eq!(
            filter_detections(&features, &map, config).unwrap().len(),
            33
        );
    }
    use linear_algebra::point;
    use ndarray::array;

    fn detection(
        class: VisualFeatureClass,
        pixel: Point2<Pixel>,
    ) -> (VisualFeatureClass, DetectedVisualFeature) {
        (
            class,
            DetectedVisualFeature {
                pixel,
                confidence: 0.95,
            },
        )
    }

    fn prediction(
        pixel: Point2<Pixel>,
        jacobian: Matrix2x6<f64>,
        noise: &PredictionNoise,
    ) -> Prediction {
        Prediction {
            pixel,
            covariance: jacobian * noise.pose * jacobian.transpose()
                + Matrix2::identity() * noise.pixel_variance,
            jacobian,
        }
    }

    #[test]
    fn joint_gate_preserves_the_configured_two_dimensional_tail_probability() {
        for (features, expected) in [
            (1, 9.210000038146973),
            (2, 13.276312533583777),
            (3, 16.8114628871141),
            (4, 20.089770800934367),
            (5, 23.20875750966771),
        ] {
            assert!((joint_mahalanobis_gate(9.21, features) - expected).abs() < 1.0e-11);
        }
        for gate in [0.01, 1.0, 9.21, 100.0, 1000.0] {
            let target_tail = (-f64::from(gate) / 2.0).exp();
            for features in 1..=5 {
                let half = joint_mahalanobis_gate(gate, features) / 2.0;
                let tail = (-half).exp()
                    * (0..features)
                        .map(|k| half.powi(k as i32) / (1..=k).map(|i| i as f64).product::<f64>())
                        .sum::<f64>();
                assert!(
                    (tail / target_tail - 1.0).abs() < 1.0e-11,
                    "gate={gate} features={features} tail={tail} expected={target_tail}"
                );
            }
        }
        // exp(-g/2) rounds to one here. The small-CDF branch must still find a positive quantile.
        for gate in [f32::from_bits(1), f32::MIN_POSITIVE] {
            let confidence = -(-f64::from(gate) / 2.0).exp_m1();
            for features in 2..=5 {
                let quantile = joint_mahalanobis_gate(gate, features);
                let leading_cdf = (quantile / 2.0).powi(features as i32)
                    / (1..=features).map(|i| i as f64).product::<f64>();
                assert!(quantile > f64::from(gate));
                assert!((leading_cdf / confidence - 1.0).abs() < 1.0e-6);
            }
        }
        for features in 1..=5 {
            let quantile = joint_mahalanobis_gate(f32::MAX, features);
            assert!(quantile.is_finite() && quantile >= f64::from(f32::MAX));
        }
    }

    #[test]
    fn joint_residual_uses_dimension_calibration_without_losing_its_absolute_gate() {
        let map = LandmarkMap::new(
            &types::field_dimensions::FieldDimensions::SPL_2025,
            crate::GlobalLocalizerParameters::default().symmetry_epsilon,
        );
        let noise = PredictionNoise {
            pose: Matrix6::zeros(),
            pixel_variance: 4.0,
        };
        let parameters = FieldMarkAssociationParameters::default();
        let class = VisualFeatureClass::LSpot;
        for n in 3..=5 {
            let detections = (0..n)
                .map(|row| detection(class, point![30.0 * row as f32, 0.0]))
                .collect::<Vec<_>>();
            let marginal_gate = parameters.global_localizer.mahalanobis_gate;
            let joint_gate = joint_mahalanobis_gate(marginal_gate, n);
            for (total_error, accepted) in [
                ((f64::from(marginal_gate) + joint_gate) / 2.0, true),
                (joint_gate + 1.0, false),
            ] {
                let shift = (noise.pixel_variance * total_error / n as f64).sqrt() as f32;
                let mut predictions = vec![None; map.landmarks.len()];
                for (row, (_, detection)) in detections.iter().enumerate() {
                    predictions[map.landmarks_for_class(class)[row]] = Some(prediction(
                        point![detection.pixel.x() + shift, 0.0],
                        Matrix2x6::zeros(),
                        &noise,
                    ));
                }
                let gates =
                    prediction_log_likelihoods(&detections, &map, &predictions, marginal_gate)
                        .unwrap();
                assert!(
                    gates.rows().into_iter().all(|row| row
                        .iter()
                        .filter(|score| score.is_finite())
                        .count()
                        == 1)
                );
                let result =
                    match_predictions(&detections, &map, &predictions, &noise, &parameters);
                assert_eq!(
                    result.is_some(),
                    accepted,
                    "n={n} error={total_error} gate={joint_gate}"
                );
                if let Some(result) = result {
                    assert_eq!(result.associations.len(), n);
                }
            }
        }
    }

    #[test]
    fn covariance_volume_does_not_reward_a_distant_plausible_rival() {
        let map = LandmarkMap::new(
            &types::field_dimensions::FieldDimensions::SPL_2025,
            crate::GlobalLocalizerParameters::default().symmetry_epsilon,
        );
        let mut predictions = vec![None; map.landmarks.len()];
        let noise = PredictionNoise {
            pose: Matrix6::identity(),
            pixel_variance: 4.0,
        };
        let mut detections = Vec::with_capacity(3);
        for (index, class) in [
            VisualFeatureClass::LSpot,
            VisualFeatureClass::TSpot,
            VisualFeatureClass::PenaltySpot,
        ]
        .into_iter()
        .enumerate()
        {
            let pixel = point![100.0 + index as f32 * 100.0, 100.0];
            predictions[map.landmarks_for_class(class)[0]] = Some(prediction(
                point![pixel.x() + 1.0, pixel.y() + 1.0],
                Matrix2x6::zeros(),
                &noise,
            ));
            detections.push(detection(class, pixel));
        }
        // Its Mahalanobis error (0.02) is smaller than the correct match's (0.5), solely
        // because of its enormous covariance. Gaussian uncertainty volume must penalize it.
        predictions[map.landmarks_for_class(VisualFeatureClass::LSpot)[1]] = Some(prediction(
            point![10100.0, 10100.0],
            Matrix2x6::identity() * 1.0e5,
            &noise,
        ));
        let result = match_predictions(
            &detections,
            &map,
            &predictions,
            &noise,
            &FieldMarkAssociationParameters::default(),
        )
        .unwrap();
        assert_eq!(result.associations.len(), 3);
        for (association, (class, feature)) in result.associations.iter().zip(detections) {
            assert_eq!(association.detection, feature.pixel);
            assert_eq!(
                association.field_point.xy(),
                map.landmarks[map.landmarks_for_class(class)[0]].xy
            );
        }
    }

    #[test]
    fn joint_triples_distinguish_inconsistent_neighbors_but_reject_shared_ambiguity() {
        let map = LandmarkMap::new(
            &types::field_dimensions::FieldDimensions::SPL_2025,
            crate::GlobalLocalizerParameters::default().symmetry_epsilon,
        );
        let noise = PredictionNoise {
            pose: Matrix6::identity() * 10000.0,
            pixel_variance: 4.0,
        };
        let classes = [
            VisualFeatureClass::LSpot,
            VisualFeatureClass::TSpot,
            VisualFeatureClass::PenaltySpot,
        ];
        let detections =
            classes.map(|class| detection(class, point![100.0 * class.index() as f32, 100.0]));
        let parameters = FieldMarkAssociationParameters::default();
        let mut predictions = vec![None; map.landmarks.len()];
        for (row, &(class, feature)) in detections.iter().enumerate() {
            let ids = map.landmarks_for_class(class);
            predictions[ids[0]] = Some(prediction(feature.pixel, Matrix2x6::identity(), &noise));
            predictions[ids[1]] = Some(prediction(
                point![
                    feature.pixel.x() + [8.0, -8.0, 16.0][row],
                    feature.pixel.y()
                ],
                Matrix2x6::identity(),
                &noise,
            ));
        }
        let result =
            match_predictions(&detections, &map, &predictions, &noise, &parameters).unwrap();
        for (association, class) in result.associations.iter().zip(classes) {
            assert_eq!(
                association.field_point.xy(),
                map.landmarks[map.landmarks_for_class(class)[0]].xy
            );
        }

        // Now an entire rival triple has the same shared shift: geometry cannot disambiguate it.
        for &(class, feature) in &detections {
            predictions[map.landmarks_for_class(class)[1]] = Some(prediction(
                point![feature.pixel.x() + 8.0, feature.pixel.y()],
                Matrix2x6::identity(),
                &noise,
            ));
        }
        assert!(match_predictions(&detections, &map, &predictions, &noise, &parameters).is_none());

        // For a common 8-pixel x shift, delta log L = (3 * 8^2) / (2 * (4 + 3 * 10000)).
        let mut threshold = parameters.clone();
        threshold.tracking.score_ratio = 1.002;
        assert!(match_predictions(&detections, &map, &predictions, &noise, &threshold).is_some());
        threshold.tracking.score_ratio = 1.004;
        assert!(match_predictions(&detections, &map, &predictions, &noise, &threshold).is_none());

        // Rivals beyond the output ceiling still veto; removing them would falsely certify.
        for &(class, feature) in &detections {
            for (index, shift) in [-79.0, 81.0].into_iter().enumerate() {
                predictions[map.landmarks_for_class(class)[index]] = Some(prediction(
                    point![feature.pixel.x() + shift, feature.pixel.y()],
                    Matrix2x6::identity(),
                    &noise,
                ));
            }
        }
        assert!(match_predictions(&detections, &map, &predictions, &noise, &parameters).is_none());
        for class in classes {
            predictions[map.landmarks_for_class(class)[1]] = None;
        }
        assert_eq!(
            match_predictions(&detections, &map, &predictions, &noise, &parameters)
                .unwrap()
                .associations
                .len(),
            3
        );
        let mut zero_confidence = detections;
        zero_confidence[0].1.confidence = 0.0;
        assert!(
            match_predictions(&zero_confidence, &map, &predictions, &noise, &parameters).is_none()
        );

        // A unique triple with mutually inconsistent errors must not pass on likelihood gap alone.
        let class = classes[0];
        predictions[map.landmarks_for_class(class)[0]]
            .as_mut()
            .unwrap()
            .pixel = detections[0].1.pixel;
        assert!(match_predictions(&detections, &map, &predictions, &noise, &parameters).is_none());
    }

    #[test]
    fn joint_assignments_require_distinct_landmarks() {
        let map = LandmarkMap::new(
            &types::field_dimensions::FieldDimensions::SPL_2025,
            crate::GlobalLocalizerParameters::default().symmetry_epsilon,
        );
        let noise = PredictionNoise {
            pose: Matrix6::zeros(),
            pixel_variance: 4.0,
        };
        let class = VisualFeatureClass::LSpot;
        for n in 3..=5 {
            let mut detections = (0..n - 1)
                .map(|row| detection(class, point![20.0 * row as f32, 0.0]))
                .collect::<Vec<_>>();
            let mut predictions = vec![None; map.landmarks.len()];
            for (row, &(_, feature)) in detections.iter().enumerate() {
                predictions[map.landmarks_for_class(class)[row]] =
                    Some(prediction(feature.pixel, Matrix2x6::zeros(), &noise));
            }
            // Every row has exactly one gated ID, but the last two rows need the same one.
            let mut duplicate = detections[n - 2];
            duplicate.1.pixel = point![duplicate.1.pixel.x() + 2.0, 0.0];
            detections.push(duplicate);
            assert!(
                match_predictions(
                    &detections,
                    &map,
                    &predictions,
                    &noise,
                    &FieldMarkAssociationParameters::default()
                )
                .is_none(),
                "n={n}"
            );
        }
    }

    #[test]
    fn four_and_five_shared_features_resolve_repeated_class_neighbors_not_coherent_rivals() {
        let map = LandmarkMap::new(
            &types::field_dimensions::FieldDimensions::SPL_2025,
            crate::GlobalLocalizerParameters::default().symmetry_epsilon,
        );
        let noise = PredictionNoise {
            pose: Matrix6::identity() * 10000.0,
            pixel_variance: 4.0,
        };
        let class = VisualFeatureClass::LSpot;
        let ids = map.landmarks_for_class(class);
        let parameters = FieldMarkAssociationParameters::default();
        for n in [4, 5] {
            let detections = (0..n)
                .map(|row| detection(class, point![40.0 * row as f32, 100.0]))
                .collect::<Vec<_>>();
            let mut predictions = vec![None; map.landmarks.len()];
            for (row, &(_, feature)) in detections.iter().enumerate() {
                predictions[ids[row]] =
                    Some(prediction(feature.pixel, Matrix2x6::identity(), &noise));
                predictions[ids[n + row]] = Some(prediction(
                    point![
                        feature.pixel.x() + [8.0, -8.0, 16.0, -16.0, 24.0][row],
                        feature.pixel.y()
                    ],
                    Matrix2x6::identity(),
                    &noise,
                ));
            }
            let gates = prediction_log_likelihoods(
                &detections,
                &map,
                &predictions,
                parameters.global_localizer.mahalanobis_gate,
            )
            .unwrap();
            assert!(
                gates.rows().into_iter().all(|row| row
                    .iter()
                    .filter(|score| score.is_finite())
                    .count()
                    == 2 * n)
            );
            let result =
                match_predictions(&detections, &map, &predictions, &noise, &parameters).unwrap();
            assert_eq!(result.associations.len(), n);
            for (row, association) in result.associations.iter().enumerate() {
                assert_eq!(association.detection, detections[row].1.pixel);
                assert_eq!(association.field_point.xy(), map.landmarks[ids[row]].xy);
            }
            for (row, &(_, feature)) in detections.iter().enumerate() {
                predictions[ids[n + row]] = Some(prediction(
                    point![feature.pixel.x() + 8.0, feature.pixel.y()],
                    Matrix2x6::identity(),
                    &noise,
                ));
            }
            assert!(
                match_predictions(&detections, &map, &predictions, &noise, &parameters).is_none(),
                "n={n}"
            );
        }
    }

    #[test]
    fn joint_work_exhaustion_after_a_winner_rejects_until_every_rival_is_scored() {
        let map = LandmarkMap::new(
            &types::field_dimensions::FieldDimensions::SPL_2025,
            crate::GlobalLocalizerParameters::default().symmetry_epsilon,
        );
        let noise = PredictionNoise {
            pose: Matrix6::zeros(),
            pixel_variance: 4.0,
        };
        let class = VisualFeatureClass::LSpot;
        let ids = map.landmarks_for_class(class);
        for n in 3..=5 {
            let detections = (0..n)
                .map(|row| detection(class, point![20.0 * row as f32, 0.0]))
                .collect::<Vec<_>>();
            let mut predictions = vec![None; map.landmarks.len()];
            for (row, &(_, feature)) in detections.iter().enumerate() {
                predictions[ids[row]] = Some(prediction(feature.pixel, Matrix2x6::zeros(), &noise));
            }
            let mut parameters = FieldMarkAssociationParameters::default();
            parameters.global_localizer.max_work = n;
            assert_eq!(
                match_predictions(&detections, &map, &predictions, &noise, &parameters)
                    .unwrap()
                    .associations
                    .len(),
                n
            );
            predictions[ids[n]] = Some(prediction(
                point![detections[n - 1].1.pixel.x() + 4.0, 0.0],
                Matrix2x6::zeros(),
                &noise,
            ));
            // n attempts find the exact winner, but the last row has one more gated candidate.
            assert!(
                match_predictions(&detections, &map, &predictions, &noise, &parameters).is_none()
            );
            parameters.global_localizer.max_work = n + 1;
            assert_eq!(
                match_predictions(&detections, &map, &predictions, &noise, &parameters)
                    .unwrap()
                    .associations
                    .len(),
                n
            );
            // Scoring an unusable rival also fails closed rather than retaining the earlier winner.
            predictions[ids[n]].as_mut().unwrap().jacobian[(0, 0)] = f64::NAN;
            assert!(
                match_predictions(&detections, &map, &predictions, &noise, &parameters).is_none()
            );
        }
    }

    #[test]
    fn five_feature_dense_budget_cannot_return_an_early_unique_winner() {
        let map = LandmarkMap::new(
            &types::field_dimensions::FieldDimensions::SPL_2025,
            crate::GlobalLocalizerParameters::default().symmetry_epsilon,
        );
        let noise = PredictionNoise {
            pose: Matrix6::identity() * 10000.0,
            pixel_variance: 4.0,
        };
        let class = VisualFeatureClass::LSpot;
        let ids = map.landmarks_for_class(class);
        let detections = [0.0, 40.0, 80.0, 120.0, 160.0].map(|x| detection(class, point![x, 0.0]));
        let mut predictions = vec![None; map.landmarks.len()];
        for (&column, x) in ids.iter().zip([
            0.0, 40.0, 80.0, 120.0, 160.0, 17.0, 68.0, 111.0, 145.0, 206.0, 251.0, 299.0,
        ]) {
            predictions[column] = Some(prediction(point![x, 0.0], Matrix2x6::identity(), &noise));
        }
        let mut parameters = FieldMarkAssociationParameters::default();
        let gates = prediction_log_likelihoods(
            &detections,
            &map,
            &predictions,
            parameters.global_localizer.mahalanobis_gate,
        )
        .unwrap();
        assert!(
            gates.rows().into_iter().all(|row| row
                .iter()
                .filter(|score| score.is_finite())
                .count()
                == 12)
        );
        // 12 + 12*12 + 12P2*12 + 12P3*12 + 12P4*12 attempts, including duplicate IDs.
        // The first complete assignment is the unique winner; all 95,040 must still be scored.
        for budget in [100_000, 160_139, 160_140] {
            parameters.global_localizer.max_work = budget;
            let result = match_predictions(&detections, &map, &predictions, &noise, &parameters);
            assert_eq!(result.is_some(), budget == 160_140, "budget={budget}");
            if let Some(result) = result {
                for (row, association) in result.associations.iter().enumerate() {
                    assert_eq!(association.field_point.xy(), map.landmarks[ids[row]].xy);
                }
            }
        }
    }

    #[test]
    #[ignore = "host runtime characterization; run explicitly with --ignored --nocapture"]
    fn joint_dense_runtime_characterization() {
        use std::{hint::black_box, time::Instant};

        let map = LandmarkMap::new(
            &types::field_dimensions::FieldDimensions::SPL_2025,
            crate::GlobalLocalizerParameters::default().symmetry_epsilon,
        );
        let noise = PredictionNoise {
            pose: Matrix6::identity() * 10000.0,
            pixel_variance: 4.0,
        };
        let class = VisualFeatureClass::LSpot;
        let detections = [0.0, 2.0, 4.0, 6.0, 8.0].map(|x| detection(class, point![x, 0.0]));
        let mut predictions = vec![None; map.landmarks.len()];
        for &column in map.landmarks_for_class(class) {
            predictions[column] = Some(prediction(point![0.0, 0.0], Matrix2x6::identity(), &noise));
        }
        let mut parameters = FieldMarkAssociationParameters::default();
        for (n, mixed) in [(3, false), (4, false), (5, false), (5, true)] {
            let mut selected = detections;
            if mixed {
                selected[0].0 = VisualFeatureClass::TSpot;
                for &column in map.landmarks_for_class(VisualFeatureClass::TSpot) {
                    predictions[column] =
                        Some(prediction(point![0.0, 0.0], Matrix2x6::identity(), &noise));
                }
            }
            let detections = &selected[..n];
            let gates = prediction_log_likelihoods(
                detections,
                &map,
                &predictions,
                parameters.global_localizer.mahalanobis_gate,
            )
            .unwrap();
            for (row, (class, _)) in gates.rows().into_iter().zip(detections) {
                assert_eq!(
                    row.iter().filter(|score| score.is_finite()).count(),
                    map.landmarks_for_class(*class).len()
                );
            }
            // Include dense same/mixed-class cases at both a small budget and the configured default.
            for budget in [1000, 100_000] {
                parameters.global_localizer.max_work = budget;
                let mut samples = Vec::with_capacity(100);
                for iteration in 0..110 {
                    let start = Instant::now();
                    let result = black_box(match_predictions(
                        black_box(detections),
                        black_box(&map),
                        black_box(&predictions),
                        black_box(&noise),
                        black_box(&parameters),
                    ));
                    let elapsed = start.elapsed();
                    assert!(
                        result.is_none(),
                        "all complete assignments tie, or budget is exhausted"
                    );
                    if iteration >= 10 {
                        samples.push(elapsed);
                    }
                }
                samples.sort_unstable();
                eprintln!(
                    "joint/dense-{n} mixed={mixed} budget={budget}: samples={} p50={:?} p95={:?} p99={:?} max={:?}",
                    samples.len(),
                    samples[50],
                    samples[95],
                    samples[99],
                    samples[99]
                );
            }
        }
    }

    #[test]
    fn single_prior_covariance_matches_finite_differences_and_coherent_two_pose_model() {
        use nalgebra::{Matrix2x6, Translation3, UnitQuaternion};
        use types::{field_dimensions::FieldDimensions, visual_localization::AssociationGeometry};

        let pose = |translation: [f32; 3], angles: [f32; 3]| {
            nalgebra::Isometry3::from_parts(
                Translation3::from(Vector3::from(translation)),
                UnitQuaternion::from_euler_angles(angles[0], angles[1], angles[2]),
            )
        };
        let local = pose([-1.0, 0.5, 0.7], [0.1, -0.15, 0.6]);
        let alignment = pose([0.7, -0.2, 0.0], [0.0, 0.0, 0.5]);
        let field_pose = alignment * local;
        let factor = Matrix6::from_fn(|r, c| {
            if r == c {
                0.02 + r as f32 * 0.005
            } else if c < r {
                0.01 * (r + c + 1) as f32 / 6.0
            } else {
                0.0
            }
        });
        let covariance = factor * factor.transpose();
        let estimate = PoseEstimate {
            pose: Isometry3::wrap(field_pose.cast()),
            covariance: covariance.cast(),
        };
        let geometry = AssociationGeometry {
            generation: 0,
            epoch: 0,
            estimate,
            last_successful_solve: ros_z::time::Time::zero(),
        };
        let features = crate::DetectedVisualFeatures::default();
        let input = AssociationInput {
            visual_features: &features,
            robot_to_camera: Isometry3::wrap(pose([0.15, 0.07, 0.08], [3.0, 0.05, -0.1]).inverse()),
            geometry: &geometry,
            camera_intrinsic: projection::intrinsic::Intrinsic::new(
                nalgebra::vector![430.0, 520.0],
                point![320.0, 240.0],
            ),
            field_dimensions: &FieldDimensions::SPL_2025,
            time: ros_z::time::Time::zero(),
        };
        let point = point![0.5, -0.3, 0.0];
        let parameters = FieldMarkAssociationParameters::default();
        let process = process_covariance(estimate.pose, 0.7, &parameters);
        let noise = PredictionNoise {
            pose: covariance.cast::<f64>() + process,
            pixel_variance: 2.3_f64.powi(2),
        };
        let projected = project_landmark(point, input, &noise).unwrap();
        let pixel = projected.pixel;
        let actual = projected.covariance;
        let project = |pose: nalgebra::Isometry3<f64>| {
            let p = input.robot_to_camera.inner.cast::<f64>()
                * pose.inverse()
                * point.inner.cast::<f64>();
            nalgebra::vector![430.0 * p.x / p.z + 320.0, 520.0 * p.y / p.z + 240.0]
        };
        let jacobian = Matrix2x6::<f64>::from_columns(&std::array::from_fn::<_, 6, _>(|axis| {
            let sample = |step| {
                let mut tangent = Vector3::<f64>::zeros();
                tangent[axis % 3] = step;
                let perturbation = if axis < 3 {
                    nalgebra::Isometry3::rotation(tangent)
                } else {
                    nalgebra::Isometry3::translation(tangent.x, tangent.y, tangent.z)
                };
                project(field_pose.cast::<f64>() * perturbation)
            };
            (sample(1.0e-5) - sample(-1.0e-5)) / 2.0e-5
        }));
        assert!((projected.jacobian - jacobian).norm() < projected.jacobian.norm() * 0.0002);
        let expected = jacobian * noise.pose * jacobian.transpose()
            + Matrix2::identity() * noise.pixel_variance;
        assert!(
            (pixel.coords().inner - project(field_pose.cast::<f64>()).cast::<f32>()).norm() < 0.001
        );
        assert!(
            (actual - expected).norm() < expected.norm() * 0.0002,
            "actual {actual}, expected {expected}"
        );
        let without_correlations =
            jacobian * Matrix6::from_diagonal(&noise.pose.diagonal()) * jacobian.transpose()
                + Matrix2::identity() * noise.pixel_variance;
        assert!((actual - without_correlations).norm() > expected.norm() * 0.01);

        // Reconstruct the old coherent snapshot: both poses denote the same transform,
        // but the process floor was rotated from Local rather than Field.
        let old_process = process_covariance(Isometry3::wrap(local.cast()), 0.7, &parameters);
        assert!((process - old_process).norm() < process.norm() * 1.0e-6);
        let points = [point, point![0.2, 0.4, 0.0]];
        let projections = points.map(|p| project_landmark(p, input, &noise).unwrap());
        let old_jacobians = points.map(|p| {
            let robot = (field_pose.inverse() * p.inner).coords;
            let camera = input.robot_to_camera.inner * nalgebra::Point3::from(robot);
            let projection = Matrix2x3::new(
                430.0 / camera.z,
                0.0,
                -430.0 * camera.x / camera.z.powi(2),
                0.0,
                520.0 / camera.z,
                -520.0 * camera.y / camera.z.powi(2),
            );
            let action = Matrix3x6::from_fn(|r, c| {
                if c < 3 {
                    robot.cross_matrix()[(r, c)]
                } else if r == c - 3 {
                    -1.0
                } else {
                    0.0
                }
            });
            let anchor_to_camera =
                input.robot_to_camera.inner * (alignment * local).inverse() * field_pose;
            let ja = projection * anchor_to_camera.rotation.to_rotation_matrix().matrix() * action;
            let jc = projection
                * input
                    .robot_to_camera
                    .inner
                    .rotation
                    .to_rotation_matrix()
                    .matrix()
                * action;
            (ja.cast::<f64>(), jc.cast::<f64>())
        });
        for i in 0..2 {
            for j in 0..2 {
                let old =
                    old_jacobians[i].0 * covariance.cast::<f64>() * old_jacobians[j].0.transpose()
                        + old_jacobians[i].1 * old_process * old_jacobians[j].1.transpose();
                let combined =
                    projections[i].jacobian * noise.pose * projections[j].jacobian.transpose();
                assert!((old - combined).norm() < old.norm() * 1.0e-5);
            }
        }
    }

    #[test]
    fn assignment_is_one_to_one_and_rejects_near_tied_rematching() {
        let mut benefits = array![
            [0.9, 0.8, 0.0, 0.0, 0.0, 0.0],
            [0.89, 0.1, 0.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 0.9, 0.0, 0.0, 0.0],
        ];
        let pairs = unique_assignment(&mut benefits, 3, 1.05).unwrap();
        assert_eq!(pairs, vec![(0, 1), (1, 0), (2, 2)]);
        benefits[(1, 1)] = 0.8;
        assert!(unique_assignment(&mut benefits, 3, 1.05).is_none());
        benefits[(0, 1)] = 0.0;
        assert_eq!(
            unique_assignment(&mut benefits, 3, 1.05).unwrap(),
            vec![(0, 0), (1, 1), (2, 2)]
        );
    }
}
