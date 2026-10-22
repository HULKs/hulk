use super::*;
use hsl_network_messages::PlayerNumber;
use linear_algebra::point;

fn all_tiles_owned_by_one(field: FieldDimensions) -> SearchVoronoiSelection {
    let pose = Isometry2::<Ground, Field>::identity();
    SearchVoronoiSelection::new(
        field,
        18,
        PlayerNumber::One,
        [(pose.as_pose(), PlayerNumber::One)],
    )
}

#[test]
fn heatmap_uses_half_meter_cells() {
    assert_eq!(
        Heatmap::new(FieldDimensions::SPL_2025, 18).map.dim(),
        (18, 12)
    );
}

#[test]
fn border_penalty_prefers_interior_without_hiding_strong_border_evidence() {
    let field = FieldDimensions::SPL_2025;
    let mut heatmap = Heatmap::new(field, 18);
    let selection = all_tiles_owned_by_one(field);
    let border = (17, 11);
    let interior = (9, 5);
    heatmap.map[border] = 0.9;
    heatmap.map[interior] = 0.82;
    let mut parameters = SearchSuggestorParameters {
        minimum_search_priority: 0.4,
        border_view_penalty: 0.15,
        border_view_penalty_width_fraction: 0.5,
        observation_distance_range: 0.0..0.0,
        ..Default::default()
    };

    assert_eq!(
        heatmap.get_suggested_search_index(
            field,
            &parameters,
            None,
            &selection,
            parameters.minimum_search_priority,
            SearchArea::WholeField,
        ),
        Some(interior)
    );

    parameters.border_view_penalty = 0.0;
    assert_eq!(
        heatmap.get_suggested_search_index(
            field,
            &parameters,
            None,
            &selection,
            parameters.minimum_search_priority,
            SearchArea::WholeField,
        ),
        Some(border)
    );

    parameters.border_view_penalty = 0.15;
    heatmap.map[border] = 1.0;
    assert_eq!(
        heatmap.get_suggested_search_index(
            field,
            &parameters,
            None,
            &selection,
            parameters.minimum_search_priority,
            SearchArea::WholeField,
        ),
        Some(border)
    );
}

#[test]
fn border_penalty_changes_choice_with_turn_preference() {
    let field = FieldDimensions::SPL_2025;
    let mut heatmap = Heatmap::new(field, 18);
    let selection = all_tiles_owned_by_one(field);
    let border = (17, 11);
    let interior = (9, 5);
    heatmap.map[border] = 0.95;
    heatmap.map[interior] = 0.82;
    let mut parameters = SearchSuggestorParameters {
        minimum_search_priority: 0.4,
        border_view_penalty: 0.15,
        border_view_penalty_width_fraction: 0.5,
        turn_preference_priority_margin: 0.1,
        observation_distance_range: 0.0..0.0,
        ..Default::default()
    };
    let pose =
        Isometry2::<Ground, Field>::from_parts(vector![0.0, 0.0], -std::f32::consts::FRAC_PI_4);

    assert_eq!(
        heatmap.get_suggested_search_index(
            field,
            &parameters,
            Some(pose),
            &selection,
            parameters.minimum_search_priority,
            SearchArea::WholeField,
        ),
        Some(interior)
    );

    parameters.border_view_penalty = 0.0;
    assert_eq!(
        heatmap.get_suggested_search_index(
            field,
            &parameters,
            Some(pose),
            &selection,
            parameters.minimum_search_priority,
            SearchArea::WholeField,
        ),
        Some(border)
    );
}

#[test]
fn selected_position_stays_with_robot_inside_selected_cell() {
    let field = FieldDimensions::SPL_2025;
    let mut heatmap = Heatmap::new(field, 18);
    heatmap.last_maximum_heatmap_position = Some((10, 5));
    let robot_position: Point2<Field> = point![0.75, -0.2];

    assert_eq!(
        heatmap.selected_position(field, robot_position, SearchArea::WholeField),
        Some(robot_position)
    );
}

#[test]
fn selected_position_is_center_of_distant_cell() {
    let field = FieldDimensions::SPL_2025;
    let mut heatmap = Heatmap::new(field, 18);
    heatmap.last_maximum_heatmap_position = Some((10, 5));

    assert_eq!(
        heatmap.selected_position(field, Point2::origin(), SearchArea::WholeField),
        Some(point![0.75, -0.25])
    );
}

#[test]
fn exhausted_search_selects_best_remaining_cell_and_yields_to_regular_target() {
    let field = FieldDimensions::SPL_2025;
    let mut heatmap = Heatmap::new(field, 18);
    let parameters = SearchSuggestorParameters {
        minimum_search_priority: 0.4,
        fallback_minimum_search_priority: 0.05,
        ..Default::default()
    };
    let pose = Isometry2::<Ground, Field>::from_parts(vector![0.0, 0.0], 0.0);
    let selection = SearchVoronoiSelection::new(
        field,
        18,
        PlayerNumber::One,
        [(pose.as_pose(), PlayerNumber::One)],
    );
    heatmap.map[(12, 6)] = 0.2;
    heatmap.map[(5, 6)] = 0.15;

    assert!(!heatmap.has_regular_search_target(
        field,
        &parameters,
        &selection,
        SearchArea::WholeField
    ));
    heatmap.update_suggested_search_position_with_voronoi(
        field,
        &parameters,
        Some(pose),
        &selection,
        parameters.minimum_search_priority,
        SearchArea::WholeField,
    );
    assert_eq!(heatmap.last_maximum_heatmap_position, None);

    heatmap.update_suggested_search_position_with_voronoi(
        field,
        &parameters,
        Some(pose),
        &selection,
        parameters.fallback_minimum_search_priority,
        SearchArea::WholeField,
    );
    assert_eq!(heatmap.last_maximum_heatmap_position, Some((12, 6)));

    heatmap.map[(5, 6)] = 0.6;
    assert!(heatmap.has_regular_search_target(
        field,
        &parameters,
        &selection,
        SearchArea::WholeField
    ));
    heatmap.update_suggested_search_position_with_voronoi(
        field,
        &parameters,
        Some(pose),
        &selection,
        parameters.minimum_search_priority,
        SearchArea::WholeField,
    );
    assert_eq!(heatmap.last_maximum_heatmap_position, Some((5, 6)));
}

#[test]
fn exhausted_search_keeps_edge_target_when_scores_are_negative() {
    let field = FieldDimensions::SPL_2025;
    let mut heatmap = Heatmap::new(field, 18);
    let parameters = SearchSuggestorParameters {
        minimum_search_priority: 0.4,
        fallback_minimum_search_priority: 0.05,
        border_view_penalty: 0.15,
        border_view_penalty_width_fraction: 0.5,
        tile_switch_hysteresis: 0.7,
        ..Default::default()
    };
    let pose = Isometry2::<Ground, Field>::from_parts(vector![0.0, 0.0], 0.0);
    let selection = SearchVoronoiSelection::new(
        field,
        18,
        PlayerNumber::One,
        [(pose.as_pose(), PlayerNumber::One)],
    );
    heatmap.map[(17, 11)] = 0.1;
    heatmap.map[(16, 11)] = 0.09;

    heatmap.update_suggested_search_position_with_voronoi(
        field,
        &parameters,
        Some(pose),
        &selection,
        parameters.fallback_minimum_search_priority,
        SearchArea::WholeField,
    );
    assert_eq!(heatmap.last_maximum_heatmap_position, Some((17, 11)));

    heatmap.map[(16, 11)] = 0.105;
    heatmap.update_suggested_search_position_with_voronoi(
        field,
        &parameters,
        Some(pose),
        &selection,
        parameters.fallback_minimum_search_priority,
        SearchArea::WholeField,
    );
    assert_eq!(heatmap.last_maximum_heatmap_position, Some((17, 11)));
}

#[test]
fn own_half_search_restricts_regular_fallback_and_previous_targets() {
    let field = FieldDimensions::SPL_2025;
    let mut heatmap = Heatmap::new(field, 18);
    let parameters = SearchSuggestorParameters {
        minimum_search_priority: 0.4,
        fallback_minimum_search_priority: 0.05,
        ..Default::default()
    };
    let pose = Isometry2::<Ground, Field>::identity();
    let selection = SearchVoronoiSelection::new(
        field,
        18,
        PlayerNumber::One,
        [(pose.as_pose(), PlayerNumber::One)],
    );
    let own_index = (5, 6);
    let opponent_index = (12, 6);
    heatmap.map[own_index] = 0.6;
    heatmap.map[opponent_index] = 0.9;

    heatmap.update_suggested_search_position_with_voronoi(
        field,
        &parameters,
        Some(pose),
        &selection,
        parameters.minimum_search_priority,
        SearchArea::WholeField,
    );
    assert_eq!(heatmap.last_maximum_heatmap_position, Some(opponent_index));

    heatmap.update_suggested_search_position_with_voronoi(
        field,
        &parameters,
        Some(pose),
        &selection,
        parameters.minimum_search_priority,
        SearchArea::OwnHalf,
    );
    assert_eq!(heatmap.last_maximum_heatmap_position, Some(own_index));

    heatmap.map[own_index] = 0.2;
    assert!(!heatmap.has_regular_search_target(
        field,
        &parameters,
        &selection,
        SearchArea::OwnHalf,
    ));
    heatmap.update_suggested_search_position_with_voronoi(
        field,
        &parameters,
        Some(pose),
        &selection,
        parameters.minimum_search_priority,
        SearchArea::OwnHalf,
    );
    assert_eq!(heatmap.last_maximum_heatmap_position, None);

    heatmap.update_suggested_search_position_with_voronoi(
        field,
        &parameters,
        Some(pose),
        &selection,
        parameters.fallback_minimum_search_priority,
        SearchArea::OwnHalf,
    );
    assert_eq!(heatmap.last_maximum_heatmap_position, Some(own_index));
    assert!(
        heatmap
            .selected_position(field, Point2::origin(), SearchArea::OwnHalf)
            .is_some_and(|position| position.x() <= 0.0)
    );

    heatmap.map[own_index] = 0.0;
    heatmap.update_suggested_search_position_with_voronoi(
        field,
        &parameters,
        Some(pose),
        &selection,
        parameters.fallback_minimum_search_priority,
        SearchArea::OwnHalf,
    );
    assert_eq!(heatmap.last_maximum_heatmap_position, None);
}

#[test]
fn own_half_target_does_not_keep_a_robot_position_beyond_midfield() {
    let field = FieldDimensions {
        length: 8.95,
        ..FieldDimensions::SPL_2025
    };
    let mut heatmap = Heatmap::new(field, 17);
    // An odd longitudinal count has a cell straddling the halfway line.
    let own_index = (8, 5);
    let robot_position = point![0.01, 0.1];
    heatmap.last_maximum_heatmap_position = Some(own_index);
    assert!(heatmap_tile_contains(
        field,
        (17, 11),
        own_index,
        robot_position
    ));

    assert_eq!(
        heatmap.selected_position(field, robot_position, SearchArea::OwnHalf),
        Some(heatmap_tile_center(field, (17, 11), own_index))
    );
}
