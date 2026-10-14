use super::*;

#[test]
fn fallback_waits_five_seconds_and_requires_an_earlier_own_ball() {
    let parameters = SearchSuggestorParameters {
        minimum_validity: 0.4,
        exhausted_search_delay: Duration::from_secs(5),
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
        0.4
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
        0.4
    );
    assert_eq!(
        search_minimum_validity(
            &mut no_regular_target_since,
            start + Duration::from_secs(4),
            true,
            false,
            &parameters,
        ),
        0.4
    );
    assert_eq!(
        search_minimum_validity(
            &mut no_regular_target_since,
            start + Duration::from_secs(5),
            true,
            false,
            &parameters,
        ),
        0.05
    );

    assert_eq!(
        search_minimum_validity(
            &mut no_regular_target_since,
            start + Duration::from_secs(6),
            true,
            true,
            &parameters,
        ),
        0.4
    );
    assert_eq!(no_regular_target_since, None);
}
