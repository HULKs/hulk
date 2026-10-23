use linear_algebra::Orientation2;

use super::*;

fn site(x: f32, y: f32, heading: f32, number: PlayerNumber) -> (Pose2<Field>, PlayerNumber) {
    (
        Pose2::from_parts(point![x, y], Orientation2::new(heading)),
        number,
    )
}

fn assert_partition(
    cache: &mut SearchVoronoiCache,
    field: FieldDimensions,
    cells_x: usize,
    owner: PlayerNumber,
    sites: &[(Pose2<Field>, PlayerNumber)],
    expected_rebuilds: usize,
) {
    let actual = cache.get_or_update(field, cells_x, owner, sites.iter().copied());
    let expected = SearchVoronoiSelection::new(field, cells_x, owner, sites.iter().copied());
    assert_eq!(actual.owner, expected.owner);
    assert_eq!(actual.grid, expected.grid);
    assert_eq!(cache.rebuilds, expected_rebuilds);
}

#[test]
fn cache_reuses_partition_until_a_player_changes_voronoi_cells() {
    let field = FieldDimensions {
        length: 22.0,
        width: 14.0,
        ..FieldDimensions::SPL_2025
    };
    let mut cache = SearchVoronoiCache::default();
    let mut sites = [
        site(0.1, 0.1, 0.0, PlayerNumber::Two),
        site(3.0, 0.1, 0.0, PlayerNumber::Three),
    ];
    assert_partition(&mut cache, field, 28, PlayerNumber::Two, &sites, 1);
    assert_partition(&mut cache, field, 28, PlayerNumber::Two, &sites, 1);

    // Translation within the source cells and rotation do not change Dijkstra's inputs.
    sites[0] = site(0.2, -0.1, 1.5, PlayerNumber::Two);
    sites[1] = site(3.1, -0.1, -1.5, PlayerNumber::Three);
    assert_partition(&mut cache, field, 28, PlayerNumber::Two, &sites, 1);

    sites[0] = site(1.0, -0.1, 1.5, PlayerNumber::Two);
    assert_partition(&mut cache, field, 28, PlayerNumber::Two, &sites, 2);
    sites[1] = site(4.0, -0.1, -1.5, PlayerNumber::Three);
    assert_partition(&mut cache, field, 28, PlayerNumber::Two, &sites, 3);
}

#[test]
fn cache_invalidates_for_geometry_owner_and_membership_changes() {
    let mut field = FieldDimensions {
        length: 22.0,
        width: 14.0,
        ..FieldDimensions::SPL_2025
    };
    let mut cache = SearchVoronoiCache::default();
    let mut sites = vec![
        site(-2.0, 0.0, 0.0, PlayerNumber::Two),
        site(2.0, 0.0, 0.0, PlayerNumber::Three),
    ];
    assert_partition(&mut cache, field, 28, PlayerNumber::Two, &sites, 1);
    sites.pop();
    assert_partition(&mut cache, field, 28, PlayerNumber::Two, &sites, 2);
    sites.push(site(2.0, 0.0, 0.0, PlayerNumber::Four));
    assert_partition(&mut cache, field, 28, PlayerNumber::Two, &sites, 3);
    sites[1].1 = PlayerNumber::Three;
    assert_partition(&mut cache, field, 28, PlayerNumber::Two, &sites, 4);
    assert_partition(&mut cache, field, 28, PlayerNumber::Three, &sites, 5);
    assert_partition(&mut cache, field, 44, PlayerNumber::Three, &sites, 6);
    field.length = 21.91;
    assert_partition(&mut cache, field, 44, PlayerNumber::Three, &sites, 7);
    field.width = 13.93;
    assert_partition(&mut cache, field, 44, PlayerNumber::Three, &sites, 8);

    // Leaving and re-entering the grid must also update the partition.
    sites[1] = site(100.0, 0.0, 0.0, PlayerNumber::Three);
    assert_partition(&mut cache, field, 44, PlayerNumber::Three, &sites, 9);
    sites[1] = site(2.0, 0.0, 0.0, PlayerNumber::Three);
    assert_partition(&mut cache, field, 44, PlayerNumber::Three, &sites, 10);
}
