use std::f32::consts::FRAC_PI_2;

use ::types::field_dimensions::FieldDimensions;

use super::super::GlobalAssociator;
use super::prelude::*;
use super::test_support::*;

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
fn no_pose_hint_selects_own_half_negative_y_startup_branch() -> Result<(), String> {
    let field = FieldDimensions::SPL_2025;
    let pose_on_symmetric_branch = robot_to_field(
        field.length * 0.5 - 1.3,
        field.width * 0.5 - 0.7,
        -FRAC_PI_2,
    );

    let selected = no_hint_localization_for_pose(pose_on_symmetric_branch, 0x4e45_4759)?;
    let position = selected.inner.translation.vector;
    let (_, _, yaw) = selected.inner.rotation.euler_angles();

    assert!(
        position.x <= 0.0,
        "selected x should be in own half: {position:?}"
    );
    assert!(
        position.y < 0.0,
        "selected y should be negative: {position:?}"
    );
    assert!(yaw_error(yaw, FRAC_PI_2) < 1.0e-3, "selected yaw: {yaw}");
    Ok(())
}

#[test]
fn no_pose_hint_selects_own_half_positive_y_startup_branch() -> Result<(), String> {
    let field = FieldDimensions::SPL_2025;
    let pose_on_symmetric_branch = robot_to_field(
        field.length * 0.5 - 1.3,
        -field.width * 0.5 + 0.7,
        FRAC_PI_2,
    );

    let selected = no_hint_localization_for_pose(pose_on_symmetric_branch, 0x504f_5359)?;
    let position = selected.inner.translation.vector;
    let (_, _, yaw) = selected.inner.rotation.euler_angles();

    assert!(
        position.x <= 0.0,
        "selected x should be in own half: {position:?}"
    );
    assert!(
        position.y > 0.0,
        "selected y should be positive: {position:?}"
    );
    assert!(yaw_error(yaw, -FRAC_PI_2) < 1.0e-3, "selected yaw: {yaw}");
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
        "projected subsample stats: generated={}, solved={}, unique={}, no_result={}",
        stats.generated, stats.solved, stats.unique, stats.no_result
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

fn no_hint_localization_for_pose(
    pose: Isometry3<Robot, Field>,
    seed: u64,
) -> Result<Isometry3<Robot, Field>, String> {
    let field = FieldDimensions::SPL_2025;
    let config = GlobalAssociationConfig::default();
    let map = LandmarkMap::new(&field, config.min_map_baseline);
    let visible = projected_landmarks(&map, pose);
    let mut rng = DeterministicRng::new(seed);
    let selected = random_non_collinear_subsample(&visible, config.min_inliers, &map, &mut rng)
        .ok_or_else(|| "startup branch pose should have enough visible landmarks".to_string())?;
    let frame = synthetic_frame(&selected);
    let localizer = GlobalAssociator::new(config);
    let result = localizer
        .localize(synthetic_input(&frame.features, &field, None))
        .ok_or_else(|| "startup branch frame should localize without a pose hint".to_string())?;

    Ok(result.associations().robot_to_field)
}

fn yaw_error(yaw: f32, expected: f32) -> f32 {
    let mut error = yaw - expected;
    while error > std::f32::consts::PI {
        error -= std::f32::consts::TAU;
    }
    while error < -std::f32::consts::PI {
        error += std::f32::consts::TAU;
    }
    error.abs()
}

#[test]
#[ignore]
fn stress_projected_noisy_subsamples_never_return_wrong_associations() -> Result<(), String> {
    let stats = run_projected_subsample_cases(5_000, 0x5354_5245_5353_3031, 2.0)?;

    eprintln!(
        "projected stress stats: generated={}, solved={}, unique={}, no_result={}",
        stats.generated, stats.solved, stats.unique, stats.no_result
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
