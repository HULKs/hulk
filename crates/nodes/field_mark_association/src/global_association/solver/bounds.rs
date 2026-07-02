use super::*;

pub(super) fn optimistic_seed_score_fast(problem: &Problem, transform: Similarity2<f32>) -> f32 {
    let mut total = -problem.high_confidence_unmatched_penalty;
    for class in FEATURE_CLASSES {
        let mut values = [0.0; MAX_DETECTIONS];
        let mut value_count = 0;
        for (detection_index, _) in problem
            .detections
            .iter()
            .enumerate()
            .filter(|(_, detection)| detection.class == class)
        {
            let Some(value) = problem
                .map
                .landmarks_for_class(class)
                .iter()
                .filter_map(|&landmark_id| {
                    optimistic_edge_value(problem, transform, detection_index, landmark_id)
                })
                .max_by(f32::total_cmp)
            else {
                continue;
            };
            if value > 0.0 && value_count < values.len() {
                values[value_count] = value;
                value_count += 1;
            }
        }
        values[..value_count].sort_by(|left, right| right.total_cmp(left));
        total += values[..value_count.min(problem.map.landmarks_for_class(class).len())]
            .iter()
            .sum::<f32>();
    }
    total
}

fn optimistic_edge_value(
    problem: &Problem,
    transform: Similarity2<f32>,
    detection_index: usize,
    landmark_id: usize,
) -> Option<f32> {
    let detection = problem.detections.get(detection_index)?;
    let landmark = problem.map.landmarks.get(landmark_id)?;
    if detection.class != landmark.class {
        return None;
    }

    let predicted = transform * nalgebra::Point2::from(detection.a);
    let predicted = point![<Field>, predicted.x, predicted.y];
    let distance = (landmark.xy - predicted).inner.norm();
    if !distance.is_finite() {
        return None;
    }
    let residual_lower_bound =
        (distance - trust_region_movement_bound(problem, detection)).max(0.0);
    if residual_lower_bound > problem.cfg.association_gate {
        return None;
    }
    let accepted = Match::new(problem, detection_index, landmark_id, residual_lower_bound)?;
    Some(
        detection_evidence(problem, accepted) + unmatched_penalty_for_detection(problem, detection)
            - problem.cfg.residual_weight * residual_lower_bound.powi(2),
    )
}

fn trust_region_movement_bound(problem: &Problem, detection: &DetectionPoint) -> f32 {
    TRUST_REGION_TRANSLATION
        + detection.a.norm()
            * (TRUST_REGION_HEIGHT
                + problem.cfg.height_max * 2.0 * (0.5 * TRUST_REGION_YAW).sin().abs())
}
