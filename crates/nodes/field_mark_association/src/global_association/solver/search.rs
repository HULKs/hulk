use super::*;

pub(super) fn solve_problem(problem: &Problem) -> Option<GlobalLocalizationResult> {
    let CandidateSearch {
        mut candidates,
        truncated,
        ..
    } = candidate_hypotheses(problem);
    candidates.sort_by(compare_candidates);
    let unique_candidates = remove_equivalent_candidates(candidates, &problem.map);
    let accepted_candidates = unique_candidates
        .into_iter()
        .filter(|candidate| passes_basic_acceptance(candidate, problem.cfg))
        .collect::<Vec<_>>();
    accepted_candidates.first()?;
    if problem.detections_truncated || truncated {
        return None;
    }

    let stable = stable_candidate(problem, &accepted_candidates)?;
    let stable = oriented_candidate(problem, &stable);
    let associations = to_public(&stable, problem);
    if !robot_position_within_field_boundary(problem, associations.robot_to_field) {
        return None;
    }

    Some(GlobalLocalizationResult::UniqueModuloSymmetry(associations))
}

fn candidate_hypotheses(problem: &Problem) -> CandidateSearch {
    let mut search = CandidateSearch {
        candidates: Vec::new(),
        truncated: false,
    };
    let mut cheap = cheap_triplet_seeds(problem);
    cheap.candidates.sort_by(compare_cheap_candidates);
    let cheap_candidates = remove_equivalent_cheap_candidates(cheap.candidates, &problem.map);
    search.truncated |= cheap.truncated;

    let seeds = seed_states(problem, cheap_candidates);
    if seeds.len() > MAX_REFINED_CANDIDATES {
        search.truncated = true;
    }
    for seed in seeds.into_iter().take(MAX_REFINED_CANDIDATES) {
        refine_seed(problem, &mut search, seed);
    }

    search
}

fn remove_equivalent_cheap_candidates(
    candidates: Vec<CheapCandidate>,
    map: &LandmarkMap,
) -> Vec<CheapCandidate> {
    let mut unique = Vec::new();
    let mut keys = HashSet::new();
    for candidate in candidates {
        if !keys.insert(canonical_cheap_candidate_key(&candidate, map)) {
            continue;
        }
        unique.push(candidate);
    }
    unique
}

pub(super) fn canonical_cheap_candidate_key(
    candidate: &CheapCandidate,
    map: &LandmarkMap,
) -> AssociationKey {
    candidate
        .key
        .clone()
        .min(symmetric_cheap_candidate_key(candidate, map))
}

fn symmetric_cheap_candidate_key(candidate: &CheapCandidate, map: &LandmarkMap) -> AssociationKey {
    candidate.key.clone().symmetric(map)
}

fn seed_states(problem: &Problem, cheap_candidates: Vec<CheapCandidate>) -> Vec<SeedState> {
    let mut seeds = cheap_candidates
        .into_iter()
        .filter_map(|seed| {
            let upper_bound = optimistic_seed_score_fast(problem, seed.transform);
            upper_bound
                .is_finite()
                .then_some(SeedState { upper_bound, seed })
        })
        .collect::<Vec<_>>();
    seeds.sort_by(|left, right| right.upper_bound.total_cmp(&left.upper_bound));
    seeds
}

fn refine_seed(problem: &Problem, search: &mut CandidateSearch, seed: SeedState) {
    if let Some(candidate) = build_fitted_candidate(problem, seed.seed.transform) {
        search.candidates.push(candidate);
    }
}
