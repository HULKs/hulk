use linear_algebra::vector;

use super::*;

#[test]
fn cached_partition_tracks_freshness_and_goalkeeper_assignment_without_new_messages() {
    let field = FieldDimensions {
        length: 22.0,
        width: 14.0,
        ..FieldDimensions::SPL_2025
    };
    let state_time = Time::from_nanos(10_000_000_000);
    let own_pose = Isometry2::<Ground, Field>::identity();
    let teammate_pose = Isometry2::<Ground, Field>::from_parts(vector![3.0, 0.0], 0.0);
    let mut states = Players::new(None);
    states[PlayerNumber::Four] = Some(TimeWrapper {
        time: state_time,
        inner: PlayerState {
            pose: teammate_pose.as_pose(),
            head_yaw: 0.0,
            ball_position: None,
        },
    });
    let mut cache = SearchVoronoiCache::default();

    for (now, window, goalkeeper, includes_teammate) in [
        (
            state_time,
            Some(Duration::from_secs(1)),
            PlayerNumber::One,
            true,
        ),
        (
            state_time + Duration::from_secs(1),
            Some(Duration::from_secs(1)),
            PlayerNumber::One,
            true,
        ),
        (
            state_time + Duration::from_secs(1) + Duration::from_nanos(1),
            Some(Duration::from_secs(1)),
            PlayerNumber::One,
            false,
        ),
        (
            state_time,
            Some(Duration::from_secs(1)),
            PlayerNumber::One,
            true,
        ),
        (state_time, None, PlayerNumber::One, false),
        (
            state_time,
            Some(Duration::from_secs(1)),
            PlayerNumber::One,
            true,
        ),
        (
            state_time,
            Some(Duration::from_secs(1)),
            PlayerNumber::Four,
            false,
        ),
    ] {
        let selection = search_voronoi_selection_from_teammates(
            &mut cache,
            field,
            28,
            Some((own_pose, PlayerNumber::Two)),
            Some(goalkeeper),
            &states,
            TeammateFreshness { now, window },
        )
        .unwrap();
        let mut expected_sites = vec![(own_pose.as_pose(), PlayerNumber::Two)];
        if includes_teammate {
            expected_sites.push((teammate_pose.as_pose(), PlayerNumber::Four));
        }
        let expected = SearchVoronoiSelection::new(field, 28, PlayerNumber::Two, expected_sites);
        assert_eq!(selection.grid, expected.grid);
    }
}
