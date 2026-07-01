use super::*;

pub(super) fn cheap_candidate(
    problem: &Problem,
    transform: Similarity2<f32>,
) -> Option<CheapCandidate> {
    if !height_is_plausible(transform.scaling(), problem.cfg) {
        return None;
    }

    let mut options = [None; MAX_DETECTIONS];
    let mut option_count = 0;
    for (detection_index, detection) in problem.detections.iter().enumerate() {
        let predicted = transform * nalgebra::Point2::from(detection.a);
        let predicted = point![<Field>, predicted.x, predicted.y];
        let Some((landmark_id, residual)) = nearest_landmark(problem, detection.class, predicted)
        else {
            continue;
        };
        if residual > problem.cfg.association_gate * CHEAP_ASSOCIATION_GATE_FACTOR {
            continue;
        }
        if let Some(accepted) = Match::new(problem, detection_index, landmark_id, residual)
            && let Some(slot) = options.get_mut(option_count)
        {
            *slot = Some(accepted);
            option_count += 1;
        }
    }

    let mut used_detection_bits = 0_u64;
    let mut used_landmark_bits = 0_u64;
    let mut inlier_count = 0;
    let mut key = AssociationKey::empty();
    let mut accepted_matches = [None; MAX_DETECTIONS];
    loop {
        let mut best_index: Option<usize> = None;
        for index in 0..option_count {
            let Some(option) = options[index] else {
                continue;
            };
            if bit_is_set(used_detection_bits, option.detection_index)
                || bit_is_set(used_landmark_bits, option.landmark_id)
            {
                continue;
            }
            if best_index.is_none_or(|best_index| {
                let best = options[best_index].expect("best index points to populated option");
                option
                    .residual
                    .total_cmp(&best.residual)
                    .then_with(|| best.confidence.total_cmp(&option.confidence))
                    .is_lt()
            }) {
                best_index = Some(index);
            }
        }

        let Some(best_index) = best_index else {
            break;
        };
        let Some(accepted) = options[best_index] else {
            break;
        };
        used_detection_bits = set_bit(used_detection_bits, accepted.detection_index);
        used_landmark_bits = set_bit(used_landmark_bits, accepted.landmark_id);
        key.push(accepted.detection_index, accepted.landmark_id);
        if let Some(entry) = accepted_matches.get_mut(inlier_count) {
            *entry = Some(accepted);
        }
        inlier_count += 1;
    }

    let metrics = candidate_metrics_from_matches(
        problem,
        accepted_matches[..inlier_count].iter().copied().flatten(),
    );
    if metrics.inlier_count < problem.cfg.min_inliers {
        return None;
    }
    key.sort();

    Some(CheapCandidate {
        score: metrics.score,
        transform,
        inlier_count: metrics.inlier_count,
        metric_rms_residual: metrics.metric_rms_residual,
        key,
    })
}

fn bit_is_set(bits: u64, index: usize) -> bool {
    index >= u64::BITS as usize || bits & (1_u64 << index) != 0
}

fn set_bit(bits: u64, index: usize) -> u64 {
    if index >= u64::BITS as usize {
        bits
    } else {
        bits | (1_u64 << index)
    }
}

fn nearest_landmark(
    problem: &Problem,
    class: VisualFeatureClass,
    predicted: Point2<Field>,
) -> Option<(usize, f32)> {
    problem
        .map
        .landmarks_for_class(class)
        .iter()
        .filter_map(|&landmark_id| {
            let landmark = problem.map.landmarks.get(landmark_id)?;
            let residual = (landmark.xy - predicted).inner.norm();
            residual.is_finite().then_some((landmark_id, residual))
        })
        .min_by(|left, right| left.1.total_cmp(&right.1))
}

pub(super) fn compare_cheap_candidates(
    left: &CheapCandidate,
    right: &CheapCandidate,
) -> std::cmp::Ordering {
    compare_candidate_metrics(left.metrics(), right.metrics())
}
