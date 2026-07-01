use super::*;

pub(super) fn build_fitted_candidate(
    problem: &Problem,
    seed: Similarity2<f32>,
) -> Option<Candidate> {
    let matches = assign_under_transform(
        problem,
        seed,
        problem.cfg.association_gate * INITIAL_ASSIGNMENT_GATE_FACTOR,
    );
    if matches.len() < 2 {
        return None;
    }
    let transform = fit_similarity(problem, &matches)?;
    if !height_is_plausible(transform.scaling(), problem.cfg)
        || !transform_within_trust_region(seed, transform)
    {
        return None;
    }

    let matches = assign_under_transform(problem, transform, problem.cfg.association_gate);
    if matches.len() < 2 {
        return None;
    }
    let transform = fit_similarity(problem, &matches)?;
    if !height_is_plausible(transform.scaling(), problem.cfg)
        || !transform_within_trust_region(seed, transform)
    {
        return None;
    }

    let candidate = candidate_from_fitted_matches(problem, transform, &matches)?;
    transform_within_trust_region(seed, candidate.transform).then_some(candidate)
}

fn transform_within_trust_region(seed: Similarity2<f32>, refined: Similarity2<f32>) -> bool {
    (refined.isometry.translation.vector - seed.isometry.translation.vector).norm()
        <= TRUST_REGION_TRANSLATION
        && refined
            .isometry
            .rotation
            .angle_to(&seed.isometry.rotation)
            .abs()
            <= TRUST_REGION_YAW
        && (refined.scaling() - seed.scaling()).abs() <= TRUST_REGION_HEIGHT
}

fn assign_under_transform(problem: &Problem, transform: Similarity2<f32>, gate: f32) -> Vec<Match> {
    let mut assignments = Vec::new();
    for class in FEATURE_CLASSES {
        assignments.extend(solve_assignment_for_class(problem, transform, gate, class));
    }
    assignments
}

fn solve_assignment_for_class(
    problem: &Problem,
    transform: Similarity2<f32>,
    gate: f32,
    class: VisualFeatureClass,
) -> Vec<Match> {
    let landmarks = problem.map.landmarks_for_class(class);
    if landmarks.is_empty() {
        return Vec::new();
    }

    let mut row_ranges = Vec::new();
    let mut options = Vec::new();
    for (detection_index, detection) in problem
        .detections
        .iter()
        .enumerate()
        .filter(|(_, detection)| detection.class == class)
    {
        let row_start = options.len();
        append_landmark_options(
            problem,
            detection_index,
            detection,
            transform,
            gate,
            landmarks,
            &mut options,
        );
        if options.len() > row_start {
            row_ranges.push(row_start..options.len());
        }
    }
    if row_ranges.is_empty() {
        return Vec::new();
    }

    let Ok(zero) = NotNan::new(0.0) else {
        return Vec::new();
    };
    let mut costs = Array2::from_elem((row_ranges.len(), landmarks.len()), zero);
    for (row, range) in row_ranges.iter().enumerate() {
        for option in &options[range.clone()] {
            let value = option.value.max(0.0);
            let Ok(cost) = NotNan::new(value) else {
                continue;
            };
            costs[(row, option.column)] = cost;
        }
    }

    AssignmentProblem::from_costs(costs)
        .solve()
        .into_iter()
        .enumerate()
        .filter_map(|(row, assignment)| {
            let assignment = assignment?;
            if assignment.cost <= 0.0 {
                return None;
            }
            options[row_ranges[row].clone()]
                .iter()
                .find(|option| option.column == assignment.to)
                .map(|option| option.accepted)
        })
        .collect()
}

fn append_landmark_options(
    problem: &Problem,
    detection_index: usize,
    detection: &DetectionPoint,
    transform: Similarity2<f32>,
    gate: f32,
    landmarks: &[usize],
    options: &mut Vec<AssignmentOption>,
) {
    let predicted = transform * nalgebra::Point2::from(detection.a);
    let predicted = point![<Field>, predicted.x, predicted.y];
    for (column, &landmark_id) in landmarks.iter().enumerate() {
        let Some(landmark) = problem.map.landmarks.get(landmark_id) else {
            continue;
        };
        let residual = (landmark.xy - predicted).inner.norm();
        if !residual.is_finite() || residual > gate {
            continue;
        }
        let Some(accepted) = Match::new(problem, detection_index, landmark_id, residual) else {
            continue;
        };
        options.push(AssignmentOption {
            accepted,
            column,
            value: assignment_value(problem, accepted),
        });
    }
}

fn assignment_value(problem: &Problem, accepted: Match) -> f32 {
    let unmatched_penalty = problem
        .detections
        .get(accepted.detection_index)
        .map_or(0.0, |detection| {
            unmatched_penalty_for_detection(problem, detection)
        });
    detection_evidence(problem, accepted) + unmatched_penalty
        - problem.cfg.residual_weight * accepted.residual.powi(2)
}

pub(super) fn candidate_from_matches(problem: &Problem, matches: &[Match]) -> Option<Candidate> {
    let transform = fit_similarity(problem, matches)?;
    if !height_is_plausible(transform.scaling(), problem.cfg) {
        return None;
    }
    candidate_from_fitted_matches(problem, transform, matches)
}

fn candidate_from_fitted_matches(
    problem: &Problem,
    transform: Similarity2<f32>,
    matches: &[Match],
) -> Option<Candidate> {
    let matches = refresh_matches(problem, transform, matches, problem.cfg.association_gate);
    if matches.len() < 2 {
        return None;
    }
    let transform = fit_similarity(problem, &matches)?;
    if !height_is_plausible(transform.scaling(), problem.cfg) {
        return None;
    }
    let matches = refresh_matches(problem, transform, &matches, problem.cfg.association_gate);
    candidate_from_transform(problem, transform, matches)
}

fn refresh_matches(
    problem: &Problem,
    transform: Similarity2<f32>,
    matches: &[Match],
    gate: f32,
) -> Vec<Match> {
    matches
        .iter()
        .filter_map(|accepted| {
            let detection = problem.detections.get(accepted.detection_index)?;
            let landmark = problem.map.landmarks.get(accepted.landmark_id)?;
            let predicted = transform * nalgebra::Point2::from(detection.a);
            let predicted = point![<Field>, predicted.x, predicted.y];
            let residual = (landmark.xy - predicted).inner.norm();
            (residual <= gate)
                .then(|| {
                    Match::new(
                        problem,
                        accepted.detection_index,
                        accepted.landmark_id,
                        residual,
                    )
                })
                .flatten()
        })
        .collect()
}

fn candidate_from_transform(
    problem: &Problem,
    transform: Similarity2<f32>,
    matches: Vec<Match>,
) -> Option<Candidate> {
    if matches.is_empty() {
        return None;
    }

    let metrics = candidate_metrics_from_matches(problem, matches.iter().copied());

    Some(Candidate {
        score: metrics.score,
        matches,
        metric_rms_residual: metrics.metric_rms_residual,
        transform,
    })
}

pub(super) fn candidate_metrics_from_matches(
    problem: &Problem,
    matches: impl IntoIterator<Item = Match>,
) -> CandidateMetrics {
    let mut inlier_count = 0;
    let mut metric_total_cost = 0.0;
    let mut matched_evidence = 0.0;
    let mut matched_unmatched_penalty = 0.0;
    for accepted in matches {
        let Some(detection) = problem.detections.get(accepted.detection_index) else {
            continue;
        };
        inlier_count += 1;
        metric_total_cost += accepted.residual.powi(2);
        matched_evidence += detection_evidence(problem, accepted);
        matched_unmatched_penalty += unmatched_penalty_for_detection(problem, detection);
    }
    let unmatched_penalty =
        (problem.high_confidence_unmatched_penalty - matched_unmatched_penalty).max(0.0);
    CandidateMetrics {
        score: matched_evidence
            - problem.cfg.residual_weight * metric_total_cost
            - unmatched_penalty,
        inlier_count,
        metric_rms_residual: if inlier_count == 0 {
            f32::INFINITY
        } else {
            (metric_total_cost / inlier_count as f32).sqrt()
        },
    }
}

pub(super) fn detection_evidence(problem: &Problem, accepted: Match) -> f32 {
    accepted.confidence * problem.map.rarity_weight(accepted.class)
}

pub(super) fn unmatched_penalty_for_detection(
    problem: &Problem,
    detection: &DetectionPoint,
) -> f32 {
    if detection.confidence >= UNMATCHED_HIGH_CONFIDENCE {
        UNMATCHED_EVIDENCE_PENALTY * detection_priority(detection, &problem.map)
    } else {
        0.0
    }
}

pub(super) fn fit_similarity(problem: &Problem, matches: &[Match]) -> Option<Similarity2<f32>> {
    if matches.len() < 2 {
        return None;
    }

    let mut weight_sum = 0.0;
    let mut source_sum = Vector2::zeros();
    let mut target_sum = Vector2::zeros();
    for accepted in matches {
        let detection = problem.detections.get(accepted.detection_index)?;
        let landmark = problem.map.landmarks.get(accepted.landmark_id)?;
        let weight = detection_evidence(problem, *accepted).max(1.0e-6);
        weight_sum += weight;
        source_sum += weight * detection.a;
        target_sum += weight * landmark.xy.coords().inner;
    }
    if weight_sum <= 0.0 {
        return None;
    }
    let source_center = source_sum / weight_sum;
    let target_center = target_sum / weight_sum;

    let mut sin_sum = 0.0;
    let mut cos_sum = 0.0;
    let mut source_variance = 0.0;
    for accepted in matches {
        let detection = problem.detections.get(accepted.detection_index)?;
        let landmark = problem.map.landmarks.get(accepted.landmark_id)?;
        let weight = detection_evidence(problem, *accepted).max(1.0e-6);
        let source = detection.a - source_center;
        let target = landmark.xy.coords().inner - target_center;
        sin_sum += weight * (source.x * target.y - source.y * target.x);
        cos_sum += weight * (source.x * target.x + source.y * target.y);
        source_variance += weight * source.norm_squared();
    }
    if sin_sum.abs() + cos_sum.abs() <= 1.0e-6 || source_variance <= 1.0e-6 {
        return None;
    }

    let theta = sin_sum.atan2(cos_sum);
    let rotation = nalgebra::UnitComplex::new(theta);
    let mut scale_numerator = 0.0;
    for accepted in matches {
        let detection = problem.detections.get(accepted.detection_index)?;
        let landmark = problem.map.landmarks.get(accepted.landmark_id)?;
        let weight = detection_evidence(problem, *accepted).max(1.0e-6);
        let source = detection.a - source_center;
        let target = landmark.xy.coords().inner - target_center;
        scale_numerator += weight * (rotation * source).dot(&target);
    }

    let scale = scale_numerator / source_variance;
    if !scale.is_finite() || scale <= 0.0 {
        return None;
    }
    let translation = target_center - scale * (rotation * source_center);
    Some(Similarity2::new(translation, theta, scale))
}

pub(super) fn height_is_plausible(height: f32, cfg: GlobalAssociationConfig) -> bool {
    height.is_finite() && height >= cfg.height_min && height <= cfg.height_max
}

pub(super) fn compare_candidates(left: &Candidate, right: &Candidate) -> std::cmp::Ordering {
    compare_candidate_metrics(left.metrics(), right.metrics())
}

pub(super) fn compare_candidate_metrics(
    left: CandidateMetrics,
    right: CandidateMetrics,
) -> std::cmp::Ordering {
    right
        .score
        .total_cmp(&left.score)
        .then_with(|| right.inlier_count.cmp(&left.inlier_count))
        .then_with(|| {
            left.metric_rms_residual
                .total_cmp(&right.metric_rms_residual)
        })
}
