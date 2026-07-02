use std::f32::consts::{FRAC_PI_2, PI};

use ::types::field_dimensions::{FieldDimensions, Half, Side};
use linear_algebra::{IntoTransform, point};

use super::super::GlobalAssociator;

use super::*;

#[test]
fn stable_matches_drop_non_symmetric_repeated_class_alternative() -> Result<(), String> {
    let map = LandmarkMap::new(&FieldDimensions::SPL_2025, 0.25);
    let l_goal_box = landmark_id(&map, VisualFeatureClass::LSpot, -3.9, -1.1)?;
    let l_penalty_box = landmark_id(&map, VisualFeatureClass::LSpot, -2.85, -2.0)?;
    let l_corner = landmark_id(&map, VisualFeatureClass::LSpot, 4.5, 3.0)?;
    let t_goal_box = landmark_id(&map, VisualFeatureClass::TSpot, -4.5, -1.1)?;

    let best = candidate(vec![
        (0, l_goal_box, VisualFeatureClass::LSpot),
        (1, l_penalty_box, VisualFeatureClass::LSpot),
        (2, l_corner, VisualFeatureClass::LSpot),
        (3, t_goal_box, VisualFeatureClass::TSpot),
    ]);
    let alternative = candidate(vec![
        (0, map.symmetric_id(l_goal_box), VisualFeatureClass::LSpot),
        (
            1,
            map.symmetric_id(l_penalty_box),
            VisualFeatureClass::LSpot,
        ),
        (2, l_penalty_box, VisualFeatureClass::LSpot),
        (3, map.symmetric_id(t_goal_box), VisualFeatureClass::TSpot),
    ]);

    let near_optimal = vec![&best, &alternative];
    let stable = stable_matches_from_candidates(&best, &near_optimal, &map);
    let stable_detections = stable
        .iter()
        .map(|accepted| accepted.detection_index)
        .collect::<Vec<_>>();

    assert_eq!(stable_detections, vec![0, 1, 3]);
    Ok(())
}

#[test]
fn stable_matches_reject_mixed_symmetry_branch() -> Result<(), String> {
    let map = LandmarkMap::new(&FieldDimensions::SPL_2025, 0.25);
    let l_goal_box = landmark_id(&map, VisualFeatureClass::LSpot, -3.9, -1.1)?;
    let l_penalty_box = landmark_id(&map, VisualFeatureClass::LSpot, -2.85, -2.0)?;
    let l_corner = landmark_id(&map, VisualFeatureClass::LSpot, 4.5, 3.0)?;
    let t_goal_box = landmark_id(&map, VisualFeatureClass::TSpot, -4.5, -1.1)?;

    let best = candidate(vec![
        (0, l_goal_box, VisualFeatureClass::LSpot),
        (1, l_penalty_box, VisualFeatureClass::LSpot),
        (2, l_corner, VisualFeatureClass::LSpot),
        (3, t_goal_box, VisualFeatureClass::TSpot),
    ]);
    let mixed = candidate(vec![
        (0, l_goal_box, VisualFeatureClass::LSpot),
        (
            1,
            map.symmetric_id(l_penalty_box),
            VisualFeatureClass::LSpot,
        ),
        (2, l_penalty_box, VisualFeatureClass::LSpot),
        (3, t_goal_box, VisualFeatureClass::TSpot),
    ]);

    let near_optimal = vec![&best, &mixed];
    let stable = stable_matches_from_candidates(&best, &near_optimal, &map);
    let stable_detections = stable
        .iter()
        .map(|accepted| accepted.detection_index)
        .collect::<Vec<_>>();

    assert_eq!(stable_detections, vec![0, 3]);
    Ok(())
}

#[test]
fn oriented_candidate_prefers_yaw_branch_under_drifted_pose_hint() -> Result<(), String> {
    let field = FieldDimensions::SPL_2025;
    let config = GlobalAssociationConfig::default();
    let map = LandmarkMap::new(&field, config.min_map_baseline);
    let pose = robot_to_field(-0.2, 0.15, 0.08);
    let drifted_pose_hint = robot_to_field(-3.0, -2.0, 0.08);
    let selected = selected_goal_side_features(&map, &field, pose)?;
    let frame = synthetic_frame(&selected);
    let problem = Problem::new(
        synthetic_input(&frame.features, &field, Some(drifted_pose_hint)),
        config,
    )
    .ok_or_else(|| "synthetic problem should be valid".to_string())?;
    let symmetric = candidate_from_problem_truth(&problem, &frame, &map, true)?;

    let resolved = oriented_candidate(&problem, &symmetric);

    assert_candidate_matches_truth(&resolved, &problem, &frame)?;
    Ok(())
}

#[test]
fn pose_hint_selects_symmetry_branch_before_lock() -> Result<(), String> {
    let field = FieldDimensions::SPL_2025;
    let config = GlobalAssociationConfig::default();
    let map = LandmarkMap::new(&field, config.min_map_baseline);
    let startup_branch_pose = robot_to_field(
        -field.length * 0.5 + 1.3,
        -field.width * 0.5 + 0.7,
        FRAC_PI_2,
    );
    let visible = projected_landmarks(&map, startup_branch_pose);
    let mut rng = DeterministicRng::new(0x494e_4954_4252_414e);
    let selected = random_non_collinear_subsample(&visible, config.min_inliers, &map, &mut rng)
        .ok_or_else(|| {
            "startup-side pose should have visible non-collinear landmarks".to_string()
        })?;
    let frame = synthetic_frame(&selected);
    let localizer = GlobalAssociator::new(config);

    let result = localizer
        .localize(synthetic_input(
            &frame.features,
            &field,
            Some(startup_branch_pose),
        ))
        .ok_or_else(|| "startup pose should produce a symmetry-resolved result".to_string())?;

    assert_feature_associations_have_truth(
        &result.associations().features,
        &frame,
        &map,
        0,
        "startup",
    )?;
    Ok(())
}

#[test]
fn center_goal_side_projected_features_never_return_wrong_associations() -> Result<(), String> {
    let field = FieldDimensions::SPL_2025;
    let map = LandmarkMap::new(&field, GlobalAssociationConfig::default().min_map_baseline);
    let pose = robot_to_field(-0.2, 0.15, 0.08);
    let mut rng = DeterministicRng::new(0x594f_4c4f_474f_414c);
    let selected = selected_goal_side_features(&map, &field, pose)?;
    let frame = synthetic_frame(&add_pixel_noise(selected, &mut rng, 0.75));
    let localizer = GlobalAssociator::new(GlobalAssociationConfig::default());

    let Some(result) = localizer.localize(synthetic_input(&frame.features, &field, Some(pose)))
    else {
        return Err("goal-side projected features should produce a result".to_string());
    };

    assert_no_incorrect_associations(&result, &frame, &map, "goal-side deterministic case")?;
    assert!(
        result.associations().features.len() >= GlobalAssociationConfig::default().min_inliers,
        "goal-side deterministic case returned too few associations: {:?}",
        result.associations()
    );
    Ok(())
}

#[test]
fn projected_noisy_subsamples_never_return_wrong_associations() -> Result<(), String> {
    let stats = run_projected_subsample_cases(128, 0x5355_4253_414d_504c, 1.5)?;

    eprintln!(
        "projected subsample stats: generated={}, solved={}, unique={}, ambiguous={}, no_result={}",
        stats.generated, stats.solved, stats.unique, stats.ambiguous, stats.no_result
    );
    assert!(
        stats.generated >= 100,
        "synthetic generator produced too few valid cases: {stats:?}"
    );
    assert!(
        stats.solved >= stats.generated / 3,
        "solver produced too few results for valid projected cases: {stats:?}"
    );
    Ok(())
}

#[test]
#[ignore]
fn stress_projected_noisy_subsamples_never_return_wrong_associations() -> Result<(), String> {
    let stats = run_projected_subsample_cases(5_000, 0x5354_5245_5353_3031, 2.0)?;

    eprintln!(
        "projected stress stats: generated={}, solved={}, unique={}, ambiguous={}, no_result={}",
        stats.generated, stats.solved, stats.unique, stats.ambiguous, stats.no_result
    );
    assert!(
        stats.generated >= 4_500,
        "synthetic generator produced too few stress cases: {stats:?}"
    );
    assert!(
        stats.solved >= stats.generated / 3,
        "solver produced too few stress results: {stats:?}"
    );
    Ok(())
}

fn candidate(matches: Vec<(usize, usize, VisualFeatureClass)>) -> Candidate {
    let matches = matches
        .into_iter()
        .map(|(detection_index, landmark_id, class)| Match {
            detection_index,
            landmark_id,
            class,
            confidence: 0.9,
            residual: 0.0,
        })
        .collect::<Vec<_>>();
    Candidate {
        score: matches.len() as f32,
        matches,
        metric_rms_residual: 0.0,
        transform: Similarity2::new(Vector2::zeros(), 0.0, 0.5),
    }
}

fn candidate_from_problem_truth(
    problem: &Problem,
    frame: &SyntheticFrame,
    map: &LandmarkMap,
    symmetric: bool,
) -> Result<Candidate, String> {
    problem
        .detections
        .iter()
        .enumerate()
        .map(|(detection_index, detection)| {
            let truth_id = *frame
                .truth_by_detection_id
                .get(detection.id)
                .ok_or_else(|| {
                    format!(
                        "detection id {} was not generated by the synthetic frame",
                        detection.id
                    )
                })?;
            let landmark_id = if symmetric {
                map.symmetric_id(truth_id)
            } else {
                truth_id
            };
            Ok((detection_index, landmark_id, detection.class))
        })
        .collect::<Result<Vec<_>, _>>()
        .map(candidate)
}

fn assert_candidate_matches_truth(
    candidate: &Candidate,
    problem: &Problem,
    frame: &SyntheticFrame,
) -> Result<(), String> {
    for accepted in &candidate.matches {
        let detection = problem
            .detections
            .get(accepted.detection_index)
            .ok_or_else(|| format!("missing detection index {}", accepted.detection_index))?;
        let truth_id = frame
            .truth_by_detection_id
            .get(detection.id)
            .ok_or_else(|| {
                format!(
                    "detection id {} was not generated by the synthetic frame",
                    detection.id
                )
            })?;
        if accepted.landmark_id != *truth_id {
            return Err(format!(
                "detection {} resolved to landmark {}, expected {}",
                detection.id, accepted.landmark_id, truth_id
            ));
        }
    }
    Ok(())
}

fn selected_goal_side_features(
    map: &LandmarkMap,
    field: &FieldDimensions,
    pose: Isometry3<Robot, Field>,
) -> Result<Vec<ProjectedSyntheticFeature>, String> {
    let landmark_ids = [
        landmark_id_at(
            map,
            VisualFeatureClass::GoalPost,
            field.goal_post(Half::Opponent, Side::Left),
        )?,
        landmark_id_at(
            map,
            VisualFeatureClass::GoalPost,
            field.goal_post(Half::Opponent, Side::Right),
        )?,
        landmark_id_at(
            map,
            VisualFeatureClass::LSpot,
            field.goal_box_corner(Half::Opponent, Side::Left),
        )?,
        landmark_id_at(
            map,
            VisualFeatureClass::LSpot,
            field.goal_box_corner(Half::Opponent, Side::Right),
        )?,
        landmark_id_at(
            map,
            VisualFeatureClass::LSpot,
            field.penalty_box_corner(Half::Opponent, Side::Left),
        )?,
        landmark_id_at(
            map,
            VisualFeatureClass::PenaltySpot,
            field.penalty_spot(Half::Opponent),
        )?,
    ];
    let selected = projected_landmarks(map, pose)
        .into_iter()
        .filter(|feature| landmark_ids.contains(&feature.landmark_id))
        .collect::<Vec<_>>();
    if selected.len() != landmark_ids.len() {
        return Err(format!(
            "expected all targeted goal-side landmarks to be visible, got {} of {}",
            selected.len(),
            landmark_ids.len()
        ));
    }
    Ok(selected)
}

fn landmark_id(
    map: &LandmarkMap,
    class: VisualFeatureClass,
    x: f32,
    y: f32,
) -> Result<usize, String> {
    map.landmarks
        .iter()
        .find(|landmark| {
            landmark.class == class
                && (landmark.xy.x() - x).abs() < 1.0e-4
                && (landmark.xy.y() - y).abs() < 1.0e-4
        })
        .map(|landmark| landmark.id)
        .ok_or_else(|| format!("missing landmark {class:?} at ({x}, {y})"))
}

fn landmark_id_at(
    map: &LandmarkMap,
    class: VisualFeatureClass,
    point: Point2<Field>,
) -> Result<usize, String> {
    landmark_id(map, class, point.x(), point.y())
}

#[derive(Clone, Copy, Debug)]
struct ProjectedSyntheticFeature {
    landmark_id: usize,
    class: VisualFeatureClass,
    pixel: Point2<Pixel>,
}

#[derive(Debug)]
struct SyntheticFrame {
    features: DetectedVisualFeatures,
    truth_by_detection_id: Vec<usize>,
}

#[derive(Debug, Default)]
struct SyntheticStats {
    generated: usize,
    solved: usize,
    unique: usize,
    ambiguous: usize,
    no_result: usize,
}

struct DeterministicRng {
    state: u64,
}

impl DeterministicRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u32(&mut self) -> u32 {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        (self.state >> 32) as u32
    }

    fn f32(&mut self) -> f32 {
        self.next_u32() as f32 / (u32::MAX as f32 + 1.0)
    }

    fn f32_range(&mut self, min: f32, max: f32) -> f32 {
        min + (max - min) * self.f32()
    }

    fn usize_below(&mut self, upper: usize) -> usize {
        debug_assert!(upper > 0);
        self.next_u32() as usize % upper
    }

    fn shuffle<T>(&mut self, items: &mut [T]) {
        for index in (1..items.len()).rev() {
            let swap_with = self.usize_below(index + 1);
            items.swap(index, swap_with);
        }
    }
}

fn run_projected_subsample_cases(
    target_cases: usize,
    seed: u64,
    noise_px: f32,
) -> Result<SyntheticStats, String> {
    let field = FieldDimensions::SPL_2025;
    let config = GlobalAssociationConfig::default();
    let localizer = GlobalAssociator::new(config);
    let map = LandmarkMap::new(&field, config.min_map_baseline);
    let mut rng = DeterministicRng::new(seed);
    let mut poses = deterministic_poses(&field);
    let mut stats = SyntheticStats::default();
    let mut attempts = 0;

    while stats.generated < target_cases && attempts < target_cases * 30 {
        attempts += 1;
        let pose = poses
            .pop()
            .unwrap_or_else(|| random_pose_in_and_around_field(&field, &mut rng));
        let visible = projected_landmarks(&map, pose);
        let Some(selected) =
            random_non_collinear_subsample(&visible, config.min_inliers, &map, &mut rng)
        else {
            continue;
        };
        let frame = synthetic_frame(&add_pixel_noise(selected, &mut rng, noise_px));

        stats.generated += 1;
        let result = localizer.localize(synthetic_input(&frame.features, &field, Some(pose)));
        match result {
            Some(GlobalLocalizationResult::UniqueModuloSymmetry(result)) => {
                stats.solved += 1;
                stats.unique += 1;
                assert_feature_associations_have_truth(
                    &result.features,
                    &frame,
                    &map,
                    stats.generated,
                    "unique",
                )?;
            }
            Some(GlobalLocalizationResult::Ambiguous(result)) => {
                stats.solved += 1;
                stats.ambiguous += 1;
                assert_feature_associations_have_truth(
                    &result.features,
                    &frame,
                    &map,
                    stats.generated,
                    "ambiguous",
                )?;
            }
            None => {
                stats.no_result += 1;
            }
        }
    }

    Ok(stats)
}

fn deterministic_poses(field: &FieldDimensions) -> Vec<Isometry3<Robot, Field>> {
    let half_length = field.length * 0.5;
    let half_width = field.width * 0.5;
    let positions = [
        (0.0, 0.0),
        (0.6, -0.4),
        (-0.8, 0.5),
        (half_length - 1.0, 0.0),
        (-half_length + 1.0, 0.0),
        (half_length - 1.3, half_width - 0.7),
        (half_length - 1.3, -half_width + 0.7),
        (-half_length + 1.3, half_width - 0.7),
        (-half_length + 1.3, -half_width + 0.7),
        (0.0, half_width - 0.4),
        (0.0, -half_width + 0.4),
        (half_length + 0.35, 0.0),
        (-half_length - 0.35, 0.0),
        (0.0, half_width + 0.35),
        (0.0, -half_width - 0.35),
    ];
    let yaws = [0.0, FRAC_PI_2, -FRAC_PI_2, PI, 0.35 * PI, -0.65 * PI];
    let mut poses = positions
        .into_iter()
        .flat_map(|(x, y)| yaws.into_iter().map(move |yaw| robot_to_field(x, y, yaw)))
        .collect::<Vec<_>>();
    poses.reverse();
    poses
}

#[test]
fn robot_field_boundary_includes_border_strip() {
    let field = FieldDimensions::SPL_2025;
    let problem = boundary_problem(&field);
    let x_limit = field.length * 0.5 + field.border_strip_width;
    let y_limit = field.width * 0.5 + field.border_strip_width;

    assert!(robot_position_within_field_boundary(
        &problem,
        robot_to_field(x_limit, y_limit, 0.0),
    ));
    assert!(robot_position_within_field_boundary(
        &problem,
        robot_to_field(-x_limit, -y_limit, 0.0),
    ));
    assert!(!robot_position_within_field_boundary(
        &problem,
        robot_to_field(x_limit + 0.01, 0.0, 0.0),
    ));
    assert!(!robot_position_within_field_boundary(
        &problem,
        robot_to_field(0.0, -y_limit - 0.01, 0.0),
    ));
}

fn boundary_problem(field: &FieldDimensions) -> Problem {
    let cfg = GlobalAssociationConfig::default();
    Problem {
        cfg,
        k: synthetic_camera_intrinsic(),
        ground_to_robot: Isometry3::<Ground, Robot>::identity(),
        robot_to_camera: synthetic_robot_to_camera(),
        pose_hint: None,
        map: std::sync::Arc::new(LandmarkMap::new(field, cfg.min_map_baseline)),
        detections: Vec::new(),
        detections_truncated: false,
        camera_xy_ground: Vector2::zeros(),
        field_boundary: field_boundary_limits(field).expect("field boundary should be valid"),
        high_confidence_unmatched_penalty: 0.0,
    }
}

fn random_pose_in_and_around_field(
    field: &FieldDimensions,
    rng: &mut DeterministicRng,
) -> Isometry3<Robot, Field> {
    let x = rng.f32_range(-field.length * 0.5 - 0.8, field.length * 0.5 + 0.8);
    let y = rng.f32_range(-field.width * 0.5 - 0.8, field.width * 0.5 + 0.8);
    let yaw = rng.f32_range(-PI, PI);
    robot_to_field(x, y, yaw)
}

fn robot_to_field(x: f32, y: f32, yaw: f32) -> Isometry3<Robot, Field> {
    Isometry3::<Robot, Field>::wrap(nalgebra::Isometry3::from_parts(
        Translation3::new(x, y, 0.0),
        nalgebra::UnitQuaternion::from_axis_angle(&nalgebra::Vector3::z_axis(), yaw),
    ))
}

fn synthetic_input<'a>(
    features: &'a DetectedVisualFeatures,
    field_dimensions: &'a FieldDimensions,
    pose_hint: Option<Isometry3<Robot, Field>>,
) -> GlobalLocalizationInput<'a> {
    GlobalLocalizationInput {
        visual_features: features,
        field_dimensions,
        ground_to_robot: Isometry3::<Ground, Robot>::identity(),
        robot_to_camera: synthetic_robot_to_camera(),
        camera_intrinsic: synthetic_camera_intrinsic(),
        pose_hint,
    }
}

fn synthetic_robot_to_camera() -> Isometry3<Robot, Camera> {
    nalgebra::Isometry3::translation(0.0, 0.0, 0.5).framed_transform()
}

fn synthetic_camera_intrinsic() -> Intrinsic {
    Intrinsic::new(nalgebra::vector![60.0, 60.0], point![<Pixel>, 640.0, 480.0])
}

fn projected_landmarks(
    map: &LandmarkMap,
    robot_to_field: Isometry3<Robot, Field>,
) -> Vec<ProjectedSyntheticFeature> {
    let field_to_camera = synthetic_robot_to_camera() * robot_to_field.inverse();
    let intrinsic = synthetic_camera_intrinsic();
    map.landmarks
        .iter()
        .filter_map(|landmark| {
            let camera_point = field_to_camera * landmark.xy.extend(0.0);
            if !camera_point.z().is_finite() || camera_point.z() <= 1.0e-4 {
                return None;
            }
            let pixel = intrinsic.project(camera_point.coords());
            pixel_inside_image(pixel, 4.0).then_some(ProjectedSyntheticFeature {
                landmark_id: landmark.id,
                class: landmark.class,
                pixel,
            })
        })
        .collect()
}

fn pixel_inside_image(pixel: Point2<Pixel>, margin: f32) -> bool {
    pixel.x().is_finite()
        && pixel.y().is_finite()
        && pixel.x() >= margin
        && pixel.x() <= 1280.0 - margin
        && pixel.y() >= margin
        && pixel.y() <= 960.0 - margin
}

fn random_non_collinear_subsample(
    visible: &[ProjectedSyntheticFeature],
    min_count: usize,
    map: &LandmarkMap,
    rng: &mut DeterministicRng,
) -> Option<Vec<ProjectedSyntheticFeature>> {
    if visible.len() < min_count.max(3) {
        return None;
    }
    let max_count = visible.len().min(8);
    let min_count = min_count.max(3).min(max_count);
    for _ in 0..64 {
        let mut selected = visible.to_vec();
        rng.shuffle(&mut selected);
        selected.truncate(min_count + rng.usize_below(max_count - min_count + 1));
        if has_useful_class_mix(&selected, visible)
            && has_non_collinear_landmark_triplet(&selected, map)
        {
            return Some(selected);
        }
    }
    None
}

fn has_useful_class_mix(
    selected: &[ProjectedSyntheticFeature],
    visible: &[ProjectedSyntheticFeature],
) -> bool {
    class_count(selected) >= class_count(visible).min(2)
}

fn class_count(features: &[ProjectedSyntheticFeature]) -> usize {
    FEATURE_CLASSES
        .into_iter()
        .filter(|class| features.iter().any(|feature| feature.class == *class))
        .count()
}

fn has_non_collinear_landmark_triplet(
    selected: &[ProjectedSyntheticFeature],
    map: &LandmarkMap,
) -> bool {
    for first in 0..selected.len() {
        for second in 0..selected.len() {
            if second == first {
                continue;
            }
            for third in 0..selected.len() {
                if third == first || third == second {
                    continue;
                }
                let a = map.landmarks[selected[first].landmark_id].xy;
                let b = map.landmarks[selected[second].landmark_id].xy;
                let c = map.landmarks[selected[third].landmark_id].xy;
                let v = (b - a).inner;
                let w = (c - a).inner;
                let norm_squared = v.norm_squared();
                if norm_squared <= 0.25 {
                    continue;
                }
                let beta = (v.x * w.y - v.y * w.x) / norm_squared;
                if beta.abs() >= 0.05 {
                    return true;
                }
            }
        }
    }
    false
}

fn add_pixel_noise(
    mut selected: Vec<ProjectedSyntheticFeature>,
    rng: &mut DeterministicRng,
    noise_px: f32,
) -> Vec<ProjectedSyntheticFeature> {
    for feature in &mut selected {
        feature.pixel = point![<Pixel>,
            feature.pixel.x() + rng.f32_range(-noise_px, noise_px),
            feature.pixel.y() + rng.f32_range(-noise_px, noise_px)
        ];
    }
    selected
}

fn synthetic_frame(selected: &[ProjectedSyntheticFeature]) -> SyntheticFrame {
    let mut frame = SyntheticFrame {
        features: DetectedVisualFeatures::default(),
        truth_by_detection_id: Vec::new(),
    };
    for class in [
        VisualFeatureClass::GoalPost,
        VisualFeatureClass::LSpot,
        VisualFeatureClass::TSpot,
        VisualFeatureClass::XSpot,
        VisualFeatureClass::PenaltySpot,
    ] {
        for feature in selected.iter().filter(|feature| feature.class == class) {
            push_synthetic_feature(&mut frame.features, class, feature.pixel);
            frame.truth_by_detection_id.push(feature.landmark_id);
        }
    }
    frame
}

fn push_synthetic_feature(
    features: &mut DetectedVisualFeatures,
    class: VisualFeatureClass,
    pixel: Point2<Pixel>,
) {
    let detection = DetectedVisualFeature {
        pixel,
        confidence: 0.95,
    };
    match class {
        VisualFeatureClass::GoalPost => features.goalposts.push(detection),
        VisualFeatureClass::LSpot => features.l_spots.push(detection),
        VisualFeatureClass::TSpot => features.t_spots.push(detection),
        VisualFeatureClass::XSpot => features.x_spots.push(detection),
        VisualFeatureClass::PenaltySpot => features.penalty_spots.push(detection),
    }
}

fn assert_no_incorrect_associations(
    result: &GlobalLocalizationResult,
    frame: &SyntheticFrame,
    map: &LandmarkMap,
    case_name: &str,
) -> Result<(), String> {
    assert_feature_associations_have_truth(
        &result.associations().features,
        frame,
        map,
        0,
        case_name,
    )
}

fn assert_feature_associations_have_truth(
    associations: &[FeatureAssociation],
    frame: &SyntheticFrame,
    map: &LandmarkMap,
    case_index: usize,
    status: &str,
) -> Result<(), String> {
    for association in associations {
        let Some(&truth_id) = frame.truth_by_detection_id.get(association.detection_id) else {
            return Err(format!(
                "{status} case {case_index}: detection id {} was not generated by the synthetic frame",
                association.detection_id
            ));
        };
        if association.landmark_id != truth_id
            && association.landmark_id != map.symmetric_id(truth_id)
        {
            let expected = &map.landmarks[truth_id];
            let expected_symmetric = &map.landmarks[map.symmetric_id(truth_id)];
            let actual = &map.landmarks[association.landmark_id];
            return Err(format!(
                "{status} case {case_index}: detection {} associated with landmark {} {:?} at ({:.3}, {:.3}), expected {} or symmetric {} at ({:.3}, {:.3}) / ({:.3}, {:.3})",
                association.detection_id,
                association.landmark_id,
                actual.class,
                actual.xy.x(),
                actual.xy.y(),
                truth_id,
                expected_symmetric.id,
                expected.xy.x(),
                expected.xy.y(),
                expected_symmetric.xy.x(),
                expected_symmetric.xy.y()
            ));
        }
    }
    Ok(())
}
