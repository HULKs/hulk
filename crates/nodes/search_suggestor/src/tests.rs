use std::time::Duration;

use coordinate_systems::{Field, Ground};
use hsl_network_messages::PlayerNumber;
use linear_algebra::Isometry2;
use ros_z::time::Time;
use search_heatmap::{Heatmap, SearchArea, SearchVoronoiCache, SearchVoronoiSelection};
use types::{
    ball_position::BallPosition, field_dimensions::FieldDimensions,
    parameters::SearchSuggestorParameters, players::Players, time_wrapper::TimeWrapper,
    world_state::PlayerState,
};

use crate::{
    selection::{
        active_minimum_search_priority, ground_to_field_is_finite, search_area_for_player,
    },
    team::{TeammateFreshness, search_voronoi_selection_from_teammates, update_from_player_states},
    validation::validate_search_suggestor_parameters,
};

#[test]
fn search_partition_excludes_assigned_goalkeeper_and_keeps_its_search_independent() {
    use linear_algebra::vector;

    let field = FieldDimensions::SPL_2025;
    let now = Time::from_nanos(10_000_000_000);
    let field_pose = Isometry2::<Ground, Field>::from_parts(vector![-1.0, 0.0], 0.0);
    let keeper_pose = Isometry2::<Ground, Field>::from_parts(vector![-4.0, 0.0], 0.0);
    let other_pose = Isometry2::<Ground, Field>::from_parts(vector![3.0, 0.0], 0.0);
    let mut states = Players::new(None);
    for (number, pose) in [
        (PlayerNumber::Two, field_pose),
        (PlayerNumber::Three, keeper_pose),
        (PlayerNumber::Four, other_pose),
    ] {
        states[number] = Some(TimeWrapper {
            time: now,
            inner: PlayerState {
                pose: pose.as_pose(),
                head_yaw: 0.0,
                ball_position: None,
            },
        });
    }
    for (number, pose, expected_sites) in [
        (
            PlayerNumber::Two,
            field_pose,
            vec![
                (field_pose.as_pose(), PlayerNumber::Two),
                (other_pose.as_pose(), PlayerNumber::Four),
            ],
        ),
        (
            PlayerNumber::Three,
            keeper_pose,
            vec![(keeper_pose.as_pose(), PlayerNumber::Three)],
        ),
    ] {
        let mut cache = SearchVoronoiCache::default();
        let selection = search_voronoi_selection_from_teammates(
            &mut cache,
            field,
            18,
            Some((pose, number)),
            Some(PlayerNumber::Three),
            &states,
            TeammateFreshness {
                now,
                window: Some(Duration::from_secs(1)),
            },
        )
        .unwrap();
        let expected = SearchVoronoiSelection::new(field, 18, number, expected_sites);
        for (position, ownership) in selection.grid.cells() {
            assert_eq!(Some(ownership), expected.grid.ownership_at(position));
        }
    }
}

#[test]
fn heatmap_search_is_enabled_by_default() {
    assert!(!SearchSuggestorParameters::default().disable_heatmap_search);
}

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
            minimum_search_priority: 0.4,
            exhausted_search_delay: delay,
            fallback_minimum_search_priority: 0.05,
            ..Default::default()
        };
        let mut no_regular_target_since = None;
        let start = Time::from_nanos(1_000_000_000);

        assert_eq!(
            active_minimum_search_priority(
                &mut no_regular_target_since,
                start,
                false,
                false,
                &parameters,
            ),
            parameters.minimum_search_priority
        );
        assert_eq!(no_regular_target_since, None);

        assert_eq!(
            active_minimum_search_priority(
                &mut no_regular_target_since,
                start,
                true,
                false,
                &parameters,
            ),
            parameters.minimum_search_priority
        );
        assert_eq!(
            active_minimum_search_priority(
                &mut no_regular_target_since,
                start + delay - Duration::from_millis(1),
                true,
                false,
                &parameters,
            ),
            parameters.minimum_search_priority
        );
        assert_eq!(
            active_minimum_search_priority(
                &mut no_regular_target_since,
                start + delay,
                true,
                false,
                &parameters,
            ),
            parameters.fallback_minimum_search_priority
        );

        assert_eq!(
            active_minimum_search_priority(
                &mut no_regular_target_since,
                start + delay + Duration::from_secs(1),
                true,
                true,
                &parameters,
            ),
            parameters.minimum_search_priority
        );
        assert_eq!(no_regular_target_since, None);
    }
}

#[test]
fn rejects_invalid_fallback_thresholds() {
    let mut parameters = SearchSuggestorParameters {
        minimum_search_priority: 0.4,
        fallback_minimum_search_priority: 0.05,
        ..Default::default()
    };
    assert!(validate_search_suggestor_parameters(&parameters).is_ok());

    parameters.minimum_search_priority = -0.1;
    assert!(validate_search_suggestor_parameters(&parameters).is_err());
    parameters.minimum_search_priority = 0.4;
    parameters.fallback_minimum_search_priority = 0.5;
    assert!(validate_search_suggestor_parameters(&parameters).is_err());
    parameters.fallback_minimum_search_priority = f32::NAN;
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
fn player_state_snapshots_are_processed_once_and_removed_when_cleared() {
    use linear_algebra::{Pose2, point};

    let field_dimensions = FieldDimensions::SPL_2025;
    let parameters = SearchSuggestorParameters {
        team_ball_priority: 0.6,
        ..Default::default()
    };
    let mut heatmap = Heatmap::new(field_dimensions, 18);
    let mut last_states = Players::new(None);
    let teammate_freshness = TeammateFreshness {
        now: Time::from_nanos(1_500_000_042),
        window: Some(Duration::from_secs(2)),
    };
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
        teammate_freshness,
    );
    // The state remains useful after the one-second expiry of the shared behavior stream.
    assert!(heatmap.to_message().values.contains(&0.6));

    heatmap.set_known_ball_position(field_dimensions, point![0.0, 0.0]);
    update_from_player_states(
        &mut heatmap,
        field_dimensions,
        &states,
        &mut last_states,
        &parameters,
        teammate_freshness,
    );
    assert!(heatmap.to_message().values.contains(&1.0));
    assert!(last_states[PlayerNumber::Two].is_some());

    update_from_player_states(
        &mut heatmap,
        field_dimensions,
        &Players::new(None),
        &mut last_states,
        &parameters,
        teammate_freshness,
    );
    assert!(last_states[PlayerNumber::Two].is_none());
}

#[test]
fn stale_last_received_state_does_not_add_team_ball_priority() {
    use linear_algebra::{Pose2, point};

    let field_dimensions = FieldDimensions::SPL_2025;
    let parameters = SearchSuggestorParameters {
        team_ball_priority: 0.6,
        ..Default::default()
    };
    let mut heatmap = Heatmap::new(field_dimensions, 18);
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
        TeammateFreshness {
            now: Time::from_nanos(2_000_000_042),
            window: Some(Duration::from_secs(1)),
        },
    );

    assert!(
        heatmap
            .to_message()
            .values
            .iter()
            .all(|value| *value == 0.0)
    );
    assert!(last_states[PlayerNumber::Two].is_some());
}

#[test]
fn rejects_undefined_time_and_distance_parameters() {
    let baseline = SearchSuggestorParameters::default();
    for invalid in [
        SearchSuggestorParameters {
            cells_along_field_length: 0,
            ..baseline.clone()
        },
        SearchSuggestorParameters {
            observation_half_life_near: Duration::ZERO,
            ..baseline.clone()
        },
        SearchSuggestorParameters {
            teammate_replay_step: Duration::ZERO,
            ..baseline.clone()
        },
        SearchSuggestorParameters {
            unseen_priority_rise_time: Duration::ZERO,
            ..baseline.clone()
        },
        SearchSuggestorParameters {
            observation_distance_range: 3.0..3.0,
            ..baseline.clone()
        },
        SearchSuggestorParameters {
            last_ball_spread_speed: f32::NAN,
            ..baseline.clone()
        },
        SearchSuggestorParameters {
            teammate_observation_weight: -0.1,
            ..baseline.clone()
        },
    ] {
        assert!(validate_search_suggestor_parameters(&invalid).is_err());
    }
    assert!(
        validate_search_suggestor_parameters(&SearchSuggestorParameters {
            teammate_observation_weight: 0.0,
            last_ball_priority_fade_time: Duration::ZERO,
            ..baseline
        })
        .is_ok()
    );
}
