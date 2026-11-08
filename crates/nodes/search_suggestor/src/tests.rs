use linear_algebra::{point, vector};

use super::*;

#[test]
fn search_partition_excludes_assigned_goalkeeper_and_keeps_its_search_independent() {
    let field = FieldDimensions::SPL_2025;
    let now = Time::from_nanos(10_000_000_000);
    let field_pose = Isometry2::<Ground, Field>::from_parts(vector![-1.0, 0.0], 0.0);
    let keeper_pose = Isometry2::<Ground, Field>::from_parts(vector![-4.0, 0.0], 0.0);
    let other_pose = Isometry2::<Ground, Field>::from_parts(vector![3.0, 0.0], 0.0);
    // Use a non-default keeper number to exercise the actual role assignment.
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
        let selection = search_voronoi_selection_from_teammates(
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

fn millisecond(milliseconds: i64) -> Time {
    Time::from_nanos((1_000 + milliseconds) * 1_000_000)
}

/// The robot walks and turns; its odometry is the same as the field at the start.
fn walking_pose(milliseconds: i64) -> Pose2<Odometry> {
    let seconds = milliseconds as f32 * 1e-3;
    Pose2::new(point![0.5 * seconds, 0.0], 2.0 * seconds)
}

/// The obstacle stands still in the field, and its snapshot is held by the filter between images
/// and delayed by the fusion lag, while the only localization is about the image of 120 ms ago.
#[test]
fn occluders_of_held_obstacles_stay_in_the_field_while_the_robot_moves() {
    let odometry_to_field = Isometry2::<Odometry, Field>::from_parts(vector![-1.0, 2.0], 0.4);
    let ground_to_field_at =
        |milliseconds| odometry_to_field * walking_pose(milliseconds).as_transform::<Ground>();
    let obstacle_in_field = point![<Field>, 2.0, 1.0];
    let snapshot_time = 300;
    let snapshot = [Obstacle::robot(
        ground_to_field_at(snapshot_time).inverse() * obstacle_in_field,
        0.2,
        0.25,
    )];

    let mut alignment = GroundAlignment::default();
    let mut checked = 0;
    for milliseconds in (0..=1_000).step_by(2) {
        alignment.push_pose(millisecond(milliseconds), walking_pose(milliseconds));
        let image_time = milliseconds - 120;
        if image_time >= 0 && image_time % 32 == 0 {
            alignment.push_ground_to_field(millisecond(image_time), ground_to_field_at(image_time));
        }
        let Some(current_ground_to_field) = alignment.current_ground_to_field() else {
            continue;
        };
        if milliseconds < snapshot_time {
            continue;
        }
        let occluders = search_occluders_at(&alignment, millisecond(snapshot_time), &snapshot);
        assert_eq!(occluders.len(), 1);
        assert!((occluders[0].center - obstacle_in_field).norm() < 1e-2);
        // The newest transform places a ground position of the snapshot somewhere else entirely.
        if milliseconds >= snapshot_time + 200 {
            let mixed = current_ground_to_field * snapshot[0].position;
            assert!((mixed - obstacle_in_field).norm() > 0.5);
            checked += 1;
        }
    }
    assert!(checked > 100);
}

#[test]
fn occluders_without_a_known_ground_frame_are_left_out() {
    let mut alignment = GroundAlignment::default();
    let snapshot = [Obstacle::robot(point![1.0, 0.0], 0.2, 0.25)];

    alignment.push_pose(millisecond(0), Pose2::zero());
    assert!(search_occluders_at(&alignment, millisecond(0), &snapshot).is_empty());

    alignment.push_ground_to_field(millisecond(0), Isometry2::identity());
    assert_eq!(
        search_occluders_at(&alignment, millisecond(0), &snapshot).len(),
        1
    );

    // The robot moved in an unknown way during the outage, in which the snapshot was taken.
    alignment.push_pose(millisecond(1_000), Pose2::new(point![2.0, 0.0], 1.0));
    assert!(search_occluders_at(&alignment, millisecond(500), &snapshot).is_empty());
}

#[derive(Clone, Copy, PartialEq)]
enum DelayedInput {
    Ball,
    Hypothesis,
}

/// Runs the actual node on delayed data of a turning robot, and checks that the heatmap has its
/// peak where the input is in the field all along.
async fn assert_heatmap_marks_delayed_input_in_the_field(input: DelayedInput) {
    let layer = tempfile::tempdir().unwrap();
    std::fs::write(
        layer.path().join("search_suggestor.json5"),
        include_str!("../../../../etc/parameters/base/search_suggestor.json5"),
    )
    .unwrap();
    let context = Arc::new(
        ContextBuilder::default()
            .with_mode("peer")
            .disable_multicast_scouting()
            .with_connect_endpoints(std::iter::empty::<&str>())
            .with_listen_endpoints(["tcp/127.0.0.1:0"])
            .with_parameter_layers([layer.path().to_path_buf()])
            .build()
            .await
            .unwrap(),
    );
    let node = context.create_node("search_fixture").build().await.unwrap();
    let field = FieldDimensions::SPL_2025;
    let field_dimensions = node
        .publisher::<FieldDimensions>("field_dimensions")
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .build()
        .await
        .unwrap();
    field_dimensions.publish(&field).await.unwrap();
    let odometry = node
        .publisher::<Pose2<Odometry>>("inputs/odometry")
        .build()
        .await
        .unwrap();
    let localization = node
        .publisher::<Isometry2<Ground, Field>>("ground_to_field")
        .build()
        .await
        .unwrap();
    let balls = node
        .publisher::<Option<BallPosition<Ground>>>("ball_filter/ball_position")
        .build()
        .await
        .unwrap();
    let hypotheses = node
        .publisher::<Vec<HypotheticalBallPosition<Ground>>>(
            "ball_filter/hypothetical_ball_positions",
        )
        .build()
        .await
        .unwrap();
    let heatmaps = node
        .subscriber::<types::heatmap::Heatmap>("ball_search_heatmap")
        .build()
        .await
        .unwrap();
    let task = tokio::spawn(run_boxed(context.clone()));

    let odometry_to_field = Isometry2::<Odometry, Field>::from_parts(vector![-1.0, 2.0], 0.4);
    let ground_to_field_at =
        |milliseconds| odometry_to_field * walking_pose(milliseconds).as_transform::<Ground>();
    let ball_in_field = point![<Field>, 2.0, 1.0];
    let cell_of = |length: usize, width: usize| {
        let x = (ball_in_field.x() + field.length / 2.0) / (field.length / length as f32);
        let y = (ball_in_field.y() + field.width / 2.0) / (field.width / width as f32);
        (x as usize) * width + y as usize
    };

    let start = node.clock().now();
    let mut peaks_seen = 0;
    tokio::time::timeout(Duration::from_secs(20), async {
        let mut last_localization = None;
        while peaks_seen < 10 {
            let milliseconds = node.clock().now().duration_since(start).as_millis() as i64;
            odometry
                .publish_with_source_time(&walking_pose(milliseconds), millisecond(milliseconds))
                .await
                .unwrap();
            let image_time = milliseconds - 120;
            if image_time >= 0 && last_localization.is_none_or(|last| image_time >= last + 32) {
                localization
                    .publish_with_source_time(
                        &ground_to_field_at(image_time),
                        millisecond(image_time),
                    )
                    .await
                    .unwrap();
                last_localization = Some(image_time);
            }
            let state_time = milliseconds - 60;
            if state_time >= 0 {
                let position = ground_to_field_at(state_time).inverse() * ball_in_field;
                match input {
                    DelayedInput::Ball => balls
                        .publish_with_source_time(
                            &Some(BallPosition {
                                position,
                                velocity: vector![0.0, 0.0],
                                last_seen: millisecond(state_time),
                            }),
                            millisecond(state_time),
                        )
                        .await
                        .unwrap(),
                    DelayedInput::Hypothesis => hypotheses
                        .publish_with_source_time(
                            &vec![HypotheticalBallPosition {
                                position,
                                validity: 1.0,
                            }],
                            millisecond(state_time),
                        )
                        .await
                        .unwrap(),
                }
            }
            while heatmaps.is_ready() {
                let heatmap = heatmaps.recv().await.unwrap();
                if milliseconds < 400 {
                    continue;
                }
                let (peak, value) = heatmap
                    .values
                    .iter()
                    .copied()
                    .enumerate()
                    .max_by(|(_, a), (_, b)| a.total_cmp(b))
                    .unwrap();
                if value > 0.0 {
                    assert_eq!(
                        peak,
                        cell_of(heatmap.length as usize, heatmap.width as usize),
                        "the ball is marked at the wrong place after {milliseconds} ms"
                    );
                    peaks_seen += 1;
                }
            }
            tokio::time::sleep(Duration::from_millis(2)).await;
        }
    })
    .await
    .expect("the input should be marked in the heatmap");
    task.abort();
    let _ = task.await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn heatmap_marks_a_delayed_ball_where_it_is_in_the_field() {
    assert_heatmap_marks_delayed_input_in_the_field(DelayedInput::Ball).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn heatmap_marks_delayed_hypotheses_where_they_are_in_the_field() {
    assert_heatmap_marks_delayed_input_in_the_field(DelayedInput::Hypothesis).await;
}
