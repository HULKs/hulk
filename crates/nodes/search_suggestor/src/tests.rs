use super::*;

#[test]
fn goalkeeper_uses_own_half_search_area() {
    assert_eq!(
        search_area_for_player(PlayerNumber::One, PlayerNumber::One),
        SearchArea::OwnHalf
    );
    assert_eq!(
        search_area_for_player(PlayerNumber::Two, PlayerNumber::One),
        SearchArea::WholeField
    );
}

#[test]
fn fallback_waits_configured_delay_and_requires_an_earlier_own_ball() {
    for delay in [Duration::from_secs(2), Duration::from_secs(5)] {
        let parameters = SearchSuggestorParameters {
            minimum_validity: 0.4,
            exhausted_search_delay: delay,
            exhausted_search_minimum_validity: 0.05,
            ..Default::default()
        };
        let mut no_regular_target_since = None;
        let start = Time::from_nanos(1_000_000_000);

        assert_eq!(
            search_minimum_validity(
                &mut no_regular_target_since,
                start,
                false,
                false,
                &parameters,
            ),
            parameters.minimum_validity
        );
        assert_eq!(no_regular_target_since, None);

        assert_eq!(
            search_minimum_validity(
                &mut no_regular_target_since,
                start,
                true,
                false,
                &parameters,
            ),
            parameters.minimum_validity
        );
        assert_eq!(
            search_minimum_validity(
                &mut no_regular_target_since,
                start + delay - Duration::from_millis(1),
                true,
                false,
                &parameters,
            ),
            parameters.minimum_validity
        );
        assert_eq!(
            search_minimum_validity(
                &mut no_regular_target_since,
                start + delay,
                true,
                false,
                &parameters,
            ),
            parameters.exhausted_search_minimum_validity
        );

        assert_eq!(
            search_minimum_validity(
                &mut no_regular_target_since,
                start + delay + Duration::from_secs(1),
                true,
                true,
                &parameters,
            ),
            parameters.minimum_validity
        );
        assert_eq!(no_regular_target_since, None);
    }
}

#[test]
fn rejects_invalid_fallback_thresholds() {
    let mut parameters = SearchSuggestorParameters {
        minimum_validity: 0.4,
        exhausted_search_minimum_validity: 0.05,
        ..Default::default()
    };
    assert!(validate_search_suggestor_parameters(&parameters).is_ok());

    parameters.minimum_validity = -0.1;
    assert!(validate_search_suggestor_parameters(&parameters).is_err());
    parameters.minimum_validity = 0.4;
    parameters.exhausted_search_minimum_validity = 0.5;
    assert!(validate_search_suggestor_parameters(&parameters).is_err());
    parameters.exhausted_search_minimum_validity = f32::NAN;
    assert!(validate_search_suggestor_parameters(&parameters).is_err());
}

#[test]
fn rejects_non_finite_local_pose() {
    use linear_algebra::vector;

    assert!(ground_to_field_is_finite(
        Isometry2::<Ground, Field>::identity()
    ));
    assert!(!ground_to_field_is_finite(
        Isometry2::<Ground, Field>::from_parts(vector![f32::NAN, 0.0], 0.0)
    ));
}

#[test]
fn player_state_snapshots_are_processed_once_and_removed_on_expiry() {
    use linear_algebra::{Pose2, point};

    let field_dimensions = FieldDimensions::SPL_2025;
    let parameters = SearchSuggestorParameters {
        team_ball_weight: 0.6,
        ..Default::default()
    };
    let mut heatmap = Heatmap::new(field_dimensions);
    let mut last_states = Players::new(None);
    let states = Players {
        two: Some(TimeWrapper {
            time: Time::from_nanos(42),
            inner: PlayerState {
                pose: Pose2::default(),
                head_yaw: 0.0,
                ball_position: Some(BallPosition {
                    position: point![0.0, 0.0],
                    velocity: Default::default(),
                    last_seen: Time::from_nanos(42),
                }),
            },
        }),
        ..Players::new(None)
    };

    update_from_player_states(
        &mut heatmap,
        field_dimensions,
        &states,
        &mut last_states,
        &parameters,
        None,
        Duration::from_millis(20),
    );
    assert!(heatmap.to_message().values.contains(&0.6));

    heatmap.set_known_ball_position(field_dimensions, point![0.0, 0.0]);
    update_from_player_states(
        &mut heatmap,
        field_dimensions,
        &states,
        &mut last_states,
        &parameters,
        None,
        Duration::from_millis(20),
    );
    assert!(heatmap.to_message().values.contains(&1.0));
    assert!(last_states[PlayerNumber::Two].is_some());

    update_from_player_states(
        &mut heatmap,
        field_dimensions,
        &Players::new(None),
        &mut last_states,
        &parameters,
        None,
        Duration::from_millis(20),
    );
    assert!(last_states[PlayerNumber::Two].is_none());
}
