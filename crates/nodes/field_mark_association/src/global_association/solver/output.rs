use super::prelude::*;

pub(super) fn oriented_candidate(problem: &Problem, candidate: &Candidate) -> Candidate {
    let symmetric = symmetric_candidate(candidate, &problem.map);
    let Some(hint) = problem.pose_hint else {
        return if startup_branch_is_preferred(problem, &symmetric, candidate) {
            symmetric
        } else {
            candidate.clone()
        };
    };
    if yaw_error_to_hint(problem, &symmetric, hint) < yaw_error_to_hint(problem, candidate, hint) {
        symmetric
    } else {
        candidate.clone()
    }
}

fn startup_branch_is_preferred(problem: &Problem, left: &Candidate, right: &Candidate) -> bool {
    let left_pose = robot_to_field_for_candidate(problem, left);
    let right_pose = robot_to_field_for_candidate(problem, right);
    let left_position = left_pose.inner.translation.vector;
    let right_position = right_pose.inner.translation.vector;
    let left_is_own_half = left_position.x <= 0.0;
    let right_is_own_half = right_position.x <= 0.0;
    if left_is_own_half != right_is_own_half {
        return left_is_own_half;
    }

    let left_yaw_error = startup_yaw_error(left_pose);
    let right_yaw_error = startup_yaw_error(right_pose);
    if (left_yaw_error - right_yaw_error).abs() > 1.0e-6 {
        return left_yaw_error < right_yaw_error;
    }

    left_position.x < right_position.x
}

fn startup_yaw_error(robot_to_field: Isometry3<Robot, Field>) -> f32 {
    let position = robot_to_field.inner.translation.vector;
    let (_, _, yaw) = robot_to_field.inner.rotation.euler_angles();
    if position.y < 0.0 {
        return angle_difference(yaw, std::f32::consts::FRAC_PI_2).abs();
    }
    if position.y > 0.0 {
        return angle_difference(yaw, -std::f32::consts::FRAC_PI_2).abs();
    }

    angle_difference(yaw, std::f32::consts::FRAC_PI_2)
        .abs()
        .min(angle_difference(yaw, -std::f32::consts::FRAC_PI_2).abs())
}

fn yaw_error_to_hint(
    problem: &Problem,
    candidate: &Candidate,
    hint: Isometry3<Robot, Field>,
) -> f32 {
    yaw_difference(robot_to_field_for_candidate(problem, candidate), hint).abs()
}

fn yaw_difference(left: Isometry3<Robot, Field>, right: Isometry3<Robot, Field>) -> f32 {
    let (_, _, left_yaw) = left.inner.rotation.euler_angles();
    let (_, _, right_yaw) = right.inner.rotation.euler_angles();
    angle_difference(left_yaw, right_yaw)
}

fn angle_difference(left: f32, right: f32) -> f32 {
    let mut difference = left - right;
    while difference > std::f32::consts::PI {
        difference -= std::f32::consts::TAU;
    }
    while difference < -std::f32::consts::PI {
        difference += std::f32::consts::TAU;
    }
    difference
}

fn symmetric_candidate(candidate: &Candidate, map: &LandmarkMap) -> Candidate {
    let mut candidate = candidate.clone();
    for accepted in &mut candidate.matches {
        accepted.landmark_id = map.symmetric_id(accepted.landmark_id);
    }
    candidate.transform = Similarity2::new(
        -candidate.transform.isometry.translation.vector,
        candidate.transform.isometry.rotation.angle() + std::f32::consts::PI,
        candidate.transform.scaling(),
    );
    candidate
}

pub(super) fn to_public(candidate: &Candidate, problem: &Problem) -> FeatureAssociations {
    let robot_to_field = robot_to_field_for_candidate(problem, candidate);
    let (reprojection_rmse, total_cost) = reprojection_score(candidate, problem, robot_to_field);

    FeatureAssociations {
        robot_to_field,
        features: candidate
            .matches
            .iter()
            .filter_map(|accepted| {
                let detection = problem.detections.get(accepted.detection_index)?;
                let landmark = problem.map.landmarks.get(accepted.landmark_id)?;
                Some(FeatureAssociation {
                    detection_id: detection.id,
                    landmark_id: landmark.id,
                    detection: detection.pixel,
                    field_point: landmark.xy,
                })
            })
            .collect(),
        score: GlobalLocalizationScore {
            inliers: candidate.matches.len(),
            candidate_score: candidate.score,
            metric_rms_residual: candidate.metric_rms_residual,
            reprojection_rmse,
            total_cost,
        },
    }
}

pub(super) fn robot_position_within_field_boundary(
    problem: &Problem,
    robot_to_field: Isometry3<Robot, Field>,
) -> bool {
    let position = robot_to_field.inner.translation.vector;
    position.x.is_finite()
        && position.y.is_finite()
        && position.x.abs() <= problem.field_boundary.x
        && position.y.abs() <= problem.field_boundary.y
}

fn reprojection_score(
    candidate: &Candidate,
    problem: &Problem,
    robot_to_field: Isometry3<Robot, Field>,
) -> (f32, f32) {
    let field_to_camera =
        field_to_camera_from_robot_to_field(problem.robot_to_camera, robot_to_field);
    let mut total_cost = 0.0;
    let mut count = 0;
    for accepted in &candidate.matches {
        let Some(detection) = problem.detections.get(accepted.detection_index) else {
            return (f32::INFINITY, f32::INFINITY);
        };
        let Some(landmark) = problem.map.landmarks.get(accepted.landmark_id) else {
            return (f32::INFINITY, f32::INFINITY);
        };
        let Some(projected) = project_field_point(field_to_camera, problem.k, landmark.xy) else {
            return (f32::INFINITY, f32::INFINITY);
        };
        total_cost += (projected - detection.pixel).inner.norm_squared();
        count += 1;
    }

    if count == 0 {
        (f32::INFINITY, f32::INFINITY)
    } else {
        ((total_cost / count as f32).sqrt(), total_cost)
    }
}

fn robot_to_field_for_candidate(
    problem: &Problem,
    candidate: &Candidate,
) -> Isometry3<Robot, Field> {
    match fit_robot_to_field(problem, &candidate.matches) {
        Some(robot_to_field) => robot_to_field,
        None => robot_to_field_from_similarity(problem, candidate.transform),
    }
}

fn fit_robot_to_field(problem: &Problem, matches: &[Match]) -> Option<Isometry3<Robot, Field>> {
    let count = matches.len() as f32;
    if count < 2.0 {
        return None;
    }

    let mut ground_sum = Vector2::zeros();
    let mut field_sum = Vector2::zeros();
    for accepted in matches {
        let detection = problem.detections.get(accepted.detection_index)?;
        let landmark = problem.map.landmarks.get(accepted.landmark_id)?;
        ground_sum += detection.ground.coords().inner;
        field_sum += landmark.xy.coords().inner;
    }
    let ground_center = ground_sum / count;
    let field_center = field_sum / count;

    let mut sin_sum = 0.0;
    let mut cos_sum = 0.0;
    for accepted in matches {
        let detection = problem.detections.get(accepted.detection_index)?;
        let landmark = problem.map.landmarks.get(accepted.landmark_id)?;
        let ground = detection.ground.coords().inner - ground_center;
        let field = landmark.xy.coords().inner - field_center;
        sin_sum += ground.x * field.y - ground.y * field.x;
        cos_sum += ground.x * field.x + ground.y * field.y;
    }
    if sin_sum.abs() + cos_sum.abs() <= 1.0e-6 {
        return None;
    }

    let yaw = sin_sum.atan2(cos_sum);
    let rotation = nalgebra::UnitComplex::new(yaw);
    let translation = field_center - rotation * ground_center;
    let ground_to_field = Isometry3::<Ground, Field>::wrap(nalgebra::Isometry3::from_parts(
        Translation3::new(translation.x, translation.y, 0.0),
        rotation_z(yaw),
    ));
    Some(ground_to_field * problem.ground_to_robot.inverse())
}

fn robot_to_field_from_similarity(
    problem: &Problem,
    transform: Similarity2<f32>,
) -> Isometry3<Robot, Field> {
    let yaw = transform.isometry.rotation.angle();
    let translation = transform.isometry.translation.vector
        - transform.isometry.rotation * problem.camera_xy_ground;
    let ground_to_field = Isometry3::<Ground, Field>::wrap(nalgebra::Isometry3::from_parts(
        Translation3::new(translation.x, translation.y, 0.0),
        rotation_z(yaw),
    ));
    ground_to_field * problem.ground_to_robot.inverse()
}

pub(super) fn detailed_debug_from_result(
    result: &GlobalLocalizationResult,
    problem: &Problem,
) -> GlobalLocalizationDetailedDebug {
    let associations = result.associations();
    let field_to_camera =
        field_to_camera_from_robot_to_field(problem.robot_to_camera, associations.robot_to_field);
    let projected_pixels = problem
        .map
        .landmarks
        .iter()
        .map(|landmark| project_field_point(field_to_camera, problem.k, landmark.xy))
        .collect::<Vec<_>>();
    let accepted_feature_indices = associations
        .features
        .iter()
        .map(|association| association.landmark_id)
        .collect::<Vec<_>>();

    GlobalLocalizationDetailedDebug {
        status: GlobalLocalizationDetailedStatus::UniqueModuloSymmetry,
        robot_to_field: associations.robot_to_field,
        score: associations.score,
        detections: problem
            .detections
            .iter()
            .map(|detection| GlobalLocalizationDebugDetection {
                index: detection.id,
                class: detection.class,
                pixel: detection.pixel,
                ground: detection.ground,
            })
            .collect(),
        projected_features: problem
            .map
            .landmarks
            .iter()
            .zip(projected_pixels.iter().copied())
            .map(
                |(landmark, projected_pixel)| GlobalLocalizationDebugProjection {
                    index: landmark.id,
                    symmetric_index: landmark.symmetric_id,
                    class: landmark.class,
                    field_point: landmark.xy,
                    projected_pixel,
                    accepted: accepted_feature_indices.contains(&landmark.id),
                },
            )
            .collect(),
        associations: associations
            .features
            .iter()
            .filter_map(|association| {
                let detection = problem
                    .detections
                    .iter()
                    .find(|detection| detection.id == association.detection_id)?;
                let feature_index = association.landmark_id;
                let projected_pixel = projected_pixels.get(feature_index).copied().flatten();
                let reprojection_error_px = projected_pixel
                    .map(|projected_pixel| (projected_pixel - detection.pixel).inner.norm());

                Some(GlobalLocalizationDebugAssociation {
                    detection_index: detection.id,
                    feature_index,
                    class: detection.class,
                    detection_pixel: detection.pixel,
                    back_projected_ground: detection.ground,
                    field_point: association.field_point,
                    projected_pixel,
                    reprojection_error_px,
                })
            })
            .collect(),
    }
}

pub(super) fn field_to_camera_from_robot_to_field(
    robot_to_camera: Isometry3<Robot, Camera>,
    robot_to_field: Isometry3<Robot, Field>,
) -> Isometry3<Field, Camera> {
    robot_to_camera * robot_to_field.inverse()
}

pub(super) fn project_field_point(
    field_to_camera: Isometry3<Field, Camera>,
    intrinsic: Intrinsic,
    field_point: Point2<Field>,
) -> Option<Point2<Pixel>> {
    let camera_point = field_to_camera * field_point.extend(0.0);
    (camera_point.z().is_finite() && camera_point.z() > 1.0e-4)
        .then(|| intrinsic.project(camera_point.coords()))
}

fn rotation_z(angle: f32) -> nalgebra::UnitQuaternion<f32> {
    nalgebra::UnitQuaternion::from_axis_angle(&nalgebra::Vector3::z_axis(), angle)
}
