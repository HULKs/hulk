use super::*;

pub(super) fn remove_equivalent_candidates(
    candidates: Vec<Candidate>,
    map: &LandmarkMap,
) -> Vec<Candidate> {
    let mut unique = Vec::new();
    let mut keys = Vec::new();
    for candidate in candidates {
        let key = candidate_equivalence_key(&candidate, map);
        if key.is_empty() || keys.contains(&key) {
            continue;
        }
        keys.push(key);
        unique.push(candidate);
    }
    unique
}

pub(super) fn candidate_equivalence_key(
    candidate: &Candidate,
    map: &LandmarkMap,
) -> AssociationKey {
    let exact = candidate_key(candidate, map, false);
    let symmetric = candidate_key(candidate, map, true);
    exact.min(symmetric)
}

fn candidate_key(candidate: &Candidate, map: &LandmarkMap, symmetric: bool) -> AssociationKey {
    let mut key = AssociationKey::empty();
    for accepted in &candidate.matches {
        let landmark_id = if symmetric {
            map.symmetric_id(accepted.landmark_id)
        } else {
            accepted.landmark_id
        };
        key.push(accepted.detection_index, landmark_id);
    }
    key.sort();
    key
}

pub(super) fn stable_candidate(problem: &Problem, candidates: &[Candidate]) -> Option<Candidate> {
    let best = candidates.first()?;
    let near_optimal = near_optimal_candidates(best, candidates, problem.cfg);
    let stable_matches = stable_matches_from_candidates(best, &near_optimal, &problem.map);
    if stable_matches.len() < problem.cfg.min_inliers {
        return None;
    }

    let stable = candidate_from_matches(problem, &stable_matches)?;
    passes_basic_acceptance(&stable, problem.cfg).then_some(stable)
}

fn near_optimal_candidates<'a>(
    best: &'a Candidate,
    candidates: &'a [Candidate],
    cfg: GlobalAssociationConfig,
) -> Vec<&'a Candidate> {
    candidates
        .iter()
        .filter(|candidate| {
            std::ptr::eq(*candidate, best) || !passes_score_ratio(best, candidate, cfg)
        })
        .collect()
}

pub(super) fn stable_matches_from_candidates(
    best: &Candidate,
    candidates: &[&Candidate],
    map: &LandmarkMap,
) -> Vec<Match> {
    let mut stable = best.matches.clone();
    loop {
        let before = stable.len();
        for candidate in candidates {
            prune_to_consistent_symmetry_orbit(&mut stable, candidate, map);
        }
        if stable.len() == before {
            return stable;
        }
    }
}

fn prune_to_consistent_symmetry_orbit(
    stable: &mut Vec<Match>,
    candidate: &Candidate,
    map: &LandmarkMap,
) {
    let relations = stable
        .iter()
        .map(|accepted| symmetry_relation(map, accepted, candidate))
        .collect::<Vec<_>>();
    let exact_count = relations
        .iter()
        .filter(|relation| matches!(relation, SymmetryRelation::Exact))
        .count();
    let symmetric_count = relations
        .iter()
        .filter(|relation| matches!(relation, SymmetryRelation::Symmetric))
        .count();
    let has_invalid = relations.iter().any(|relation| {
        matches!(
            relation,
            SymmetryRelation::Missing | SymmetryRelation::Other
        )
    });
    if exact_count == 0 && symmetric_count == 0 {
        stable.clear();
        return;
    }
    if !has_invalid && (exact_count == 0 || symmetric_count == 0) {
        return;
    }

    let keep = if exact_count >= symmetric_count {
        SymmetryRelation::Exact
    } else {
        SymmetryRelation::Symmetric
    };
    retain_relations(stable, &relations, |relation| relation == keep);
}

fn retain_relations(
    stable: &mut Vec<Match>,
    relations: &[SymmetryRelation],
    mut keep: impl FnMut(SymmetryRelation) -> bool,
) {
    let mut index = 0;
    stable.retain(|_| {
        let relation = relations[index];
        index += 1;
        keep(relation)
    });
}

fn symmetry_relation(
    map: &LandmarkMap,
    accepted: &Match,
    candidate: &Candidate,
) -> SymmetryRelation {
    let Some(other) = candidate
        .matches
        .iter()
        .find(|other| other.detection_index == accepted.detection_index)
    else {
        return SymmetryRelation::Missing;
    };
    if other.landmark_id == accepted.landmark_id {
        SymmetryRelation::Exact
    } else if other.landmark_id == map.symmetric_id(accepted.landmark_id) {
        SymmetryRelation::Symmetric
    } else {
        SymmetryRelation::Other
    }
}

pub(super) fn passes_basic_acceptance(candidate: &Candidate, cfg: GlobalAssociationConfig) -> bool {
    candidate.matches.len() >= cfg.min_inliers
        && candidate.metric_rms_residual <= cfg.rms_threshold
        && candidate.transform.scaling() >= cfg.height_min
        && candidate.transform.scaling() <= cfg.height_max
        && candidate.score > cfg.min_score.max(0.0)
}

fn passes_score_ratio(best: &Candidate, second: &Candidate, cfg: GlobalAssociationConfig) -> bool {
    second.score > 0.0 && best.score / second.score >= cfg.score_ratio.max(1.0)
}
