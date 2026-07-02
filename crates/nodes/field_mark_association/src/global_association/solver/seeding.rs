use super::*;

pub(super) fn cheap_triplet_seeds(problem: &Problem) -> CheapSeedSearch {
    let mut search = CheapSeedSearch {
        candidates: Vec::new(),
        truncated: false,
    };
    let mut lookup_hits = 0;
    let mut bin_probes = 0;
    let mut transform_cells: HashMap<TransformCellKey, TransformCell> = HashMap::new();

    let DetectionTriplets {
        mut triplets,
        truncated,
    } = detection_triplets(problem);
    if truncated {
        search.truncated = true;
    }
    triplets.sort_by(compare_detection_triplet_priority);

    'triplets: for triplet in triplets {
        let min_distance = problem.cfg.height_min * triplet.distance;
        let max_distance = problem.cfg.height_max * triplet.distance;
        let alpha_bin = triplet_bin(triplet.alpha);
        let beta_bin = triplet_bin(triplet.beta);

        for alpha_offset in -TRIPLET_BIN_RADIUS..=TRIPLET_BIN_RADIUS {
            for beta_offset in -TRIPLET_BIN_RADIUS..=TRIPLET_BIN_RADIUS {
                let key = MapTripletBin {
                    classes: [
                        problem.detections[triplet.a].class,
                        problem.detections[triplet.b].class,
                        problem.detections[triplet.c].class,
                    ],
                    alpha_bin: alpha_bin.saturating_add(alpha_offset),
                    beta_bin: beta_bin.saturating_add(beta_offset),
                };
                bin_probes += 1;
                if bin_probes > MAX_TRIPLET_BIN_PROBES {
                    search.truncated = true;
                    break 'triplets;
                }
                let Some(map_triplets) = problem.map.map_triplets_by_bin.get(&key) else {
                    continue;
                };
                for map_triplet in map_triplets {
                    if map_triplet.pair_dist < min_distance
                        || map_triplet.pair_dist > max_distance
                        || (map_triplet.alpha - triplet.alpha).abs() > TRIPLET_DESCRIPTOR_TOLERANCE
                        || (map_triplet.beta - triplet.beta).abs() > TRIPLET_DESCRIPTOR_TOLERANCE
                    {
                        continue;
                    }
                    lookup_hits += 1;
                    if lookup_hits > MAX_TRIPLET_LOOKUP_HITS {
                        search.truncated = true;
                        break 'triplets;
                    }
                    let transform = similarity_from_triplet(problem, &triplet, map_triplet);
                    record_transform_cell(
                        &mut transform_cells,
                        transform,
                        triplet.priority
                            - (map_triplet.alpha - triplet.alpha).abs()
                            - (map_triplet.beta - triplet.beta).abs(),
                    );
                }
            }
        }
    }

    let mut transform_cells = transform_cells.into_iter().collect::<Vec<_>>();
    transform_cells.sort_by(|(left_key, left), (right_key, right)| {
        right
            .priority
            .total_cmp(&left.priority)
            .then_with(|| left_key.cmp(right_key))
    });
    for (_, cell) in transform_cells {
        if let Some(candidate) = cheap_candidate(problem, cell.transform) {
            search.candidates.push(candidate);
        }
    }
    retain_best_cheap_candidates(&mut search, &problem.map);

    search
}

fn detection_triplets(problem: &Problem) -> DetectionTriplets {
    let mut triplets = Vec::new();
    for first in 0..problem.detections.len() {
        for second in (first + 1)..problem.detections.len() {
            for third in (second + 1)..problem.detections.len() {
                let pairs = [
                    (first, second, third),
                    (first, third, second),
                    (second, third, first),
                ];
                let Some((a, b, c, distance)) = pairs
                    .into_iter()
                    .filter_map(|(a, b, c)| {
                        let distance = (problem.detections[b].a - problem.detections[a].a).norm();
                        distance.is_finite().then_some((a, b, c, distance))
                    })
                    .max_by(|left, right| left.3.total_cmp(&right.3))
                else {
                    continue;
                };
                if distance < problem.cfg.min_detection_baseline {
                    continue;
                }
                if let Some(triplet) = make_detection_triplet(problem, a, b, c) {
                    triplets.push(triplet);
                }
                if let Some(triplet) = make_detection_triplet(problem, b, a, c) {
                    triplets.push(triplet);
                }
            }
        }
    }
    triplets.sort_by(compare_detection_triplet_priority);
    let truncated = triplets.len() > MAX_SCANNED_DETECTION_TRIPLETS;
    triplets.truncate(MAX_SCANNED_DETECTION_TRIPLETS);
    DetectionTriplets {
        triplets,
        truncated,
    }
}

fn retain_best_cheap_candidates(search: &mut CheapSeedSearch, map: &LandmarkMap) {
    search.candidates.sort_by(compare_cheap_candidates);
    let mut keys = HashSet::new();
    search
        .candidates
        .retain(|candidate| keys.insert(canonical_cheap_candidate_key(candidate, map)));
    if search.candidates.len() > MAX_CHEAP_SEEDS {
        search.truncated = true;
        search.candidates.truncate(MAX_CHEAP_SEEDS);
    }
}

fn make_detection_triplet(
    problem: &Problem,
    a: usize,
    b: usize,
    c: usize,
) -> Option<DetectionTriplet> {
    let detection_a = problem.detections.get(a)?;
    let detection_b = problem.detections.get(b)?;
    let detection_c = problem.detections.get(c)?;
    let v = detection_b.a - detection_a.a;
    let w = detection_c.a - detection_a.a;
    let distance = v.norm();
    let norm_squared = v.norm_squared();
    if distance < problem.cfg.min_detection_baseline || norm_squared <= 1.0e-6 {
        return None;
    }
    let alpha = w.dot(&v) / norm_squared;
    let beta = (v.x * w.y - v.y * w.x) / norm_squared;
    if !alpha.is_finite() || !beta.is_finite() || beta.abs() < MIN_DETECTION_TRIPLET_ABS_BETA {
        return None;
    }
    let confidence =
        (detection_a.confidence + detection_b.confidence + detection_c.confidence) / 3.0;
    let rarity = (problem.map.rarity_weight(detection_a.class)
        + problem.map.rarity_weight(detection_b.class)
        + problem.map.rarity_weight(detection_c.class))
        / 3.0;
    Some(DetectionTriplet {
        a,
        b,
        c,
        distance,
        angle: v.y.atan2(v.x),
        alpha,
        beta,
        priority: distance * beta.abs() * confidence * rarity.max(1.0e-6),
    })
}

fn compare_detection_triplet_priority(
    left: &DetectionTriplet,
    right: &DetectionTriplet,
) -> std::cmp::Ordering {
    right.priority.total_cmp(&left.priority)
}

fn similarity_from_triplet(
    problem: &Problem,
    detection_triplet: &DetectionTriplet,
    map_triplet: &MapTriplet,
) -> Similarity2<f32> {
    let scale = map_triplet.pair_dist / detection_triplet.distance;
    let theta = map_triplet.pair_angle - detection_triplet.angle;
    let rotation = nalgebra::UnitComplex::new(theta);
    let translation = map_triplet.xy_a.coords().inner
        - scale * (rotation * problem.detections[detection_triplet.a].a);
    Similarity2::new(translation, theta, scale)
}

fn record_transform_cell(
    cells: &mut HashMap<TransformCellKey, TransformCell>,
    transform: Similarity2<f32>,
    priority: f32,
) {
    let key = canonical_transform_cell_key(transform);
    match cells.get_mut(&key) {
        Some(cell) if priority > cell.priority => {
            *cell = TransformCell {
                transform,
                priority,
            };
        }
        None => {
            cells.insert(
                key,
                TransformCell {
                    transform,
                    priority,
                },
            );
        }
        _ => {}
    }
}

fn canonical_transform_cell_key(transform: Similarity2<f32>) -> TransformCellKey {
    let symmetric = Similarity2::new(
        -transform.isometry.translation.vector,
        transform.isometry.rotation.angle() + std::f32::consts::PI,
        transform.scaling(),
    );
    transform_cell_key(transform).min(transform_cell_key(symmetric))
}

fn transform_cell_key(transform: Similarity2<f32>) -> TransformCellKey {
    let translation = transform.isometry.translation.vector;
    TransformCellKey {
        x: quantize(translation.x, TRANSFORM_CELL_TRANSLATION),
        y: quantize(translation.y, TRANSFORM_CELL_TRANSLATION),
        yaw: quantize(transform.isometry.rotation.angle(), TRANSFORM_CELL_YAW),
        log_height: quantize(transform.scaling().ln(), TRANSFORM_CELL_LOG_HEIGHT),
    }
}

fn quantize(value: f32, width: f32) -> i32 {
    (value / width)
        .round()
        .clamp(i32::MIN as f32, i32::MAX as f32) as i32
}
