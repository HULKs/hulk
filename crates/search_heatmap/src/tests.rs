use super::*;

#[test]
fn high_mean_priority_is_scaled_down_without_changing_relative_values() {
    let mut heatmap = Heatmap::new(FieldDimensions::SPL_2025, 18);
    heatmap.map.fill(1.0);
    heatmap.map[(0, 0)] = 0.5;

    heatmap.limit_mean_priority(0.5);

    let mean = heatmap.map.sum() / heatmap.map.len() as f32;
    assert!((mean - 0.5).abs() < 1e-6);
    assert!((heatmap.map[(1, 1)] / heatmap.map[(0, 0)] - 2.0).abs() < 1e-6);
}

#[test]
fn low_mean_priority_is_preserved() {
    let mut heatmap = Heatmap::new(FieldDimensions::SPL_2025, 18);
    heatmap.map.fill(0.2);

    heatmap.limit_mean_priority(0.5);

    assert!(heatmap.map.iter().all(|value| *value == 0.2));
}

#[test]
fn rounded_grid_covers_field_and_roundtrips_every_cell() {
    let mut field = FieldDimensions::SPL_2025;
    field.length = 14.0;
    field.width = 9.0;
    let heatmap = Heatmap::new(field, 30);
    assert_eq!(heatmap.map.dim(), (30, 19));
    for (index, _) in heatmap.map.indexed_iter() {
        let center = heatmap_tile_center(field, heatmap.map.dim(), index);
        assert!(center.x().abs() < 7.0 && center.y().abs() < 4.5);
        assert_eq!(heatmap.field_to_heatmap(field, center), index);
        assert!(heatmap_tile_contains(
            field,
            heatmap.map.dim(),
            index,
            center
        ));
    }
    assert_eq!(heatmap.field_to_heatmap(field, point![7.0, 4.5]), (29, 18));
    assert!(heatmap_tile_contains(
        field,
        heatmap.map.dim(),
        (29, 18),
        point![7.0, 4.5]
    ));
    assert_eq!(
        Heatmap::new(FieldDimensions::SPL_2025, 30).map.dim(),
        (30, 20)
    );
}

#[test]
fn resizing_preserves_uniform_history_and_target_location() {
    let field = FieldDimensions::SPL_2025;
    let mut heatmap = Heatmap::new(field, 18);
    heatmap.map.fill(0.35);
    heatmap.last_maximum_heatmap_position = Some((9, 6));
    let old_center = heatmap_tile_center(field, heatmap.map.dim(), (9, 6));
    heatmap.resize(field, 30);
    assert!(heatmap.map.iter().all(|value| *value == 0.35));
    assert_eq!(
        heatmap.last_maximum_heatmap_position,
        Some(heatmap.field_to_heatmap(field, old_center))
    );
}

#[test]
fn outside_ball_evidence_is_ignored_even_immediately_beyond_border() {
    let field = FieldDimensions::SPL_2025;
    let mut heatmap = Heatmap::new(field, 30);
    for point in [
        point![4.5001, 0.0],
        point![0.0, -3.0001],
        point![f32::NAN, 0.0],
    ] {
        heatmap.set_known_ball_position(field, point);
        heatmap.update_with_team_ball(field, point, &Default::default());
    }
    assert!(heatmap.map.iter().all(|value| *value == 0.0));
    assert!(ball_position_in_field(field, point![4.5, 3.0]));
}
