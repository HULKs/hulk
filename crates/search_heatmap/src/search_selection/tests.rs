use super::*;
use linear_algebra::point;

#[test]
fn heatmap_uses_half_meter_cells() {
    assert_eq!(Heatmap::new(FieldDimensions::SPL_2025).map.dim(), (18, 12));
}

#[test]
fn border_penalty_prefers_interior_without_hiding_strong_border_evidence() {
    let field = FieldDimensions::SPL_2025;
    let mut heatmap = Heatmap::new(field);
    let border = (17, 11);
    let interior = (9, 5);
    heatmap.map[border] = 0.9;
    heatmap.map[interior] = 0.82;
    let mut parameters = SearchSuggestorParameters {
        minimum_validity: 0.4,
        border_view_penalty: 0.15,
        heatmap_decay_range: 0.0..3.0,
        ..Default::default()
    };

    assert_eq!(
        heatmap.get_suggested_search_index(field, &parameters, None, None),
        Some(interior)
    );

    parameters.border_view_penalty = 0.0;
    assert_eq!(
        heatmap.get_suggested_search_index(field, &parameters, None, None),
        Some(border)
    );

    parameters.border_view_penalty = 0.15;
    heatmap.map[border] = 1.0;
    assert_eq!(
        heatmap.get_suggested_search_index(field, &parameters, None, None),
        Some(border)
    );
}

#[test]
fn border_penalty_changes_choice_with_turn_preference() {
    let field = FieldDimensions::SPL_2025;
    let mut heatmap = Heatmap::new(field);
    let border = (17, 11);
    let interior = (9, 5);
    heatmap.map[border] = 0.95;
    heatmap.map[interior] = 0.82;
    let mut parameters = SearchSuggestorParameters {
        minimum_validity: 0.4,
        border_view_penalty: 0.15,
        turn_preference_priority_margin: 0.1,
        heatmap_decay_range: 0.0..3.0,
        ..Default::default()
    };
    let pose =
        Isometry2::<Ground, Field>::from_parts(vector![0.0, 0.0], -std::f32::consts::FRAC_PI_4);

    assert_eq!(
        heatmap.get_suggested_search_index(field, &parameters, Some(pose), None),
        Some(interior)
    );

    parameters.border_view_penalty = 0.0;
    assert_eq!(
        heatmap.get_suggested_search_index(field, &parameters, Some(pose), None),
        Some(border)
    );
}

#[test]
fn selected_position_stays_with_robot_inside_selected_cell() {
    let field = FieldDimensions::SPL_2025;
    let mut heatmap = Heatmap::new(field);
    heatmap.last_maximum_heatmap_position = Some((10, 5));
    let robot_position: Point2<Field> = point![0.75, -0.2];

    assert_eq!(
        heatmap.selected_position(field, robot_position),
        Some(robot_position)
    );
}

#[test]
fn selected_position_is_center_of_distant_cell() {
    let field = FieldDimensions::SPL_2025;
    let mut heatmap = Heatmap::new(field);
    heatmap.last_maximum_heatmap_position = Some((10, 5));

    assert_eq!(
        heatmap.selected_position(field, Point2::origin()),
        Some(point![0.75, -0.25])
    );
}
