use std::sync::atomic::{AtomicBool, Ordering};

use super::*;
use hsl_network_messages::{GamePhase, Half};
use linear_algebra::{point, vector};
use ros_z::parameter::RemoteParameterClient;
use ros_z_streams::CreateAnnouncingPublisher;
use types::heatmap::{SEARCH_CELL_SIZE_TOPIC, grid_cell_size};
use types::{field_dimensions::GlobalFieldSide, filtered_game_state::FilteredGameState};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn real_search_and_behavior_nodes_start_and_share_live_cell_geometry() -> Result<()> {
    let layer = tempfile::tempdir()?;
    std::fs::write(
        layer.path().join("behavior_node.json5"),
        include_str!("../../../../../etc/parameters/base/behavior_node.json5"),
    )?;
    std::fs::write(
        layer.path().join("search_suggestor.json5"),
        include_str!("../../../../../etc/parameters/base/search_suggestor.json5"),
    )?;
    let context = Arc::new(
        ContextBuilder::default()
            .with_mode("peer")
            .disable_multicast_scouting()
            .with_connect_endpoints(std::iter::empty::<&str>())
            .with_listen_endpoints(["tcp/127.0.0.1:0"])
            .with_parameter_layers([layer.path().to_path_buf()])
            .build()
            .await?,
    );
    let fixture = Arc::new(context.create_node("search_startup_test").build().await?);
    let retained = QosProfile {
        durability: QosDurability::TransientLocal,
        history: QosHistory::KeepLast(NonZeroUsize::MIN),
        ..Default::default()
    };
    let field_pub = fixture
        .publisher::<FieldDimensions>("field_dimensions")
        .qos(retained)
        .build()
        .await?;
    let player_pub = fixture
        .publisher::<PlayerNumber>("player_number")
        .qos(retained)
        .build()
        .await?;
    let sizes = fixture
        .subscriber::<Vector2<Field>>(SEARCH_CELL_SIZE_TOPIC)
        .qos(retained)
        .build()
        .await?;
    let blackboards = fixture
        .subscriber::<Blackboard>("behavior/blackboard")
        .build()
        .await?;
    let field = FieldDimensions {
        length: 14.0,
        width: 9.0,
        ..FieldDimensions::SPL_2025
    };
    field_pub.publish(&field).await?;
    player_pub.publish(&PlayerNumber::One).await?;
    let mut nodes = tokio::task::JoinSet::new();
    nodes.spawn(search_suggestor::run_boxed(context.clone()));
    let size = tokio::time::timeout(Duration::from_secs(5), async {
        tokio::select! {
            size = sizes.recv() => size.unwrap(),
            failed = nodes.join_next() => panic!("search node exited during startup: {failed:?}"),
        }
    })
    .await?;
    assert_eq!(size, grid_cell_size(field, 28));
    // Behavior starts after publication and must receive the retained geometry.
    nodes.spawn(run(context));
    let client = RemoteParameterClient::new(fixture, "/search_suggestor")?;
    for cells in [28, 30] {
        if cells == 30 {
            let response = client
                .set_json(
                    "cells_along_field_length",
                    &30.into(),
                    layer.path().to_string_lossy().into_owned(),
                    None,
                )
                .await?;
            assert!(response.success);
        }
        let expected = grid_cell_size(field, cells);
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                tokio::select! {
                    blackboard = blackboards.recv() => {
                        if blackboard.unwrap().search_cell_size == Some(expected) { break; }
                    }
                    failed = nodes.join_next() => panic!("robot node exited: {failed:?}"),
                }
            }
        })
        .await?;
    }
    nodes.abort_all();
    while nodes.join_next().await.is_some() {}
    Ok(())
}

/// Runs the actual composer and behavior node on data of a walking and turning robot, each with
/// the delay and the source time of its producer, and checks that everything that reaches the
/// behavior tree in one world state holds together in one ground frame.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn real_composer_and_behavior_hold_delayed_data_together_in_the_current_ground_frame()
-> Result<()> {
    /// What the ground frames below may be off by, within two poses of a loaded test machine.
    const TOLERANCE: f32 = 0.08;

    let layer = tempfile::tempdir()?;
    std::fs::write(
        layer.path().join("behavior_node.json5"),
        include_str!("../../../../../etc/parameters/base/behavior_node.json5"),
    )?;
    let context = Arc::new(
        ContextBuilder::default()
            .with_mode("peer")
            .disable_multicast_scouting()
            .with_connect_endpoints(std::iter::empty::<&str>())
            .with_listen_endpoints(["tcp/127.0.0.1:0"])
            .with_parameter_layers([layer.path().to_path_buf()])
            .build()
            .await?,
    );
    let node = Arc::new(context.create_node("frame_fixture").build().await?);
    let retained = QosProfile {
        durability: QosDurability::TransientLocal,
        history: QosHistory::KeepLast(NonZeroUsize::MIN),
        ..Default::default()
    };
    let field_pub = node
        .publisher::<FieldDimensions>("field_dimensions")
        .qos(retained)
        .build()
        .await?;
    let player_pub = node
        .publisher::<PlayerNumber>("player_number")
        .qos(retained)
        .build()
        .await?;
    let primary_state_pub = node
        .publisher::<PrimaryState>("primary_state")
        .qos(retained)
        .build()
        .await?;
    let odometry_pub = node
        .announcing_publisher::<Pose2<Odometry>>("inputs/odometry")
        .await?;
    let localization_pub = node
        .publisher::<Isometry2<Ground, Field>>("ground_to_field")
        .build()
        .await?;
    let ball_pub = node
        .publisher::<Option<BallPosition<Ground>>>("ball_filter/ball_position")
        .build()
        .await?;
    let team_ball_pub = node
        .publisher::<Option<BallPosition<Field>>>("team_ball")
        .build()
        .await?;
    let visual_kick_pub = node
        .publisher::<Option<BallPosition<Ground>>>("visual_kick/ball_position")
        .build()
        .await?;
    let hypotheses_pub = node
        .publisher::<Vec<HypotheticalBallPosition<Ground>>>(
            "ball_filter/hypothetical_ball_positions",
        )
        .build()
        .await?;
    let humanoids_pub = node.publisher::<Vec<Obstacle>>("obstacles").build().await?;
    let game_pub = node
        .publisher::<FilteredGameControllerState>("filtered_game_controller_state")
        .build()
        .await?;
    let blackboards = node
        .subscriber::<Blackboard>("behavior/blackboard")
        .build()
        .await?;
    field_pub.publish(&FieldDimensions::SPL_2025).await?;
    player_pub.publish(&PlayerNumber::Two).await?;
    primary_state_pub.publish(&PrimaryState::Ready).await?;

    let mut nodes = tokio::task::JoinSet::new();
    nodes.spawn(ball_state_composer::run_boxed(context.clone()));
    nodes.spawn(run(context.clone()));

    let odometry_to_field = Isometry2::<Odometry, Field>::from_parts(vector![-1.0, 2.0], 0.4);
    let pose_at = |seconds: f32| Pose2::<Odometry>::new(point![0.5 * seconds, 0.0], 2.0 * seconds);
    let ground_to_field_at =
        move |seconds: f32| odometry_to_field * pose_at(seconds).as_transform::<Ground>();
    let ball_in_field = point![<Field>, 2.0, 1.0];
    let ball_velocity_in_field = vector![<Field>, 0.3, -0.2];
    let humanoid_in_field = point![<Field>, -1.0, -2.0];
    let goal_post_in_field = point![<Field>, 4.5, 0.8];
    let team_ball_in_field = point![<Field>, -3.0, 1.5];
    let hypothesis_in_field = point![<Field>, 1.5, -1.0];

    let own_ball = Arc::new(AtomicBool::new(true));
    let fixture = {
        let node = node.clone();
        let own_ball = own_ball.clone();
        tokio::spawn(async move {
            let start = node.clock().now();
            let at = move |seconds: f32| start + Duration::from_secs_f32(seconds);
            let mut last_slow_update = -1.0;
            loop {
                let now = node.clock().now().duration_since(start).as_secs_f32();
                odometry_pub
                    .announce(at(now))
                    .await?
                    .publish(&pose_at(now))
                    .await?;
                // Fusion lag: states are published long after the time they refer to.
                let ball_time = now - 0.03;
                if ball_time > 0.0 {
                    let ball = own_ball.load(Ordering::Relaxed).then(|| {
                        let ground_to_field = ground_to_field_at(ball_time);
                        BallPosition {
                            position: ground_to_field.inverse() * ball_in_field,
                            velocity: ground_to_field.inverse() * ball_velocity_in_field,
                            // Observation age; much older than the frame of the state.
                            last_seen: at((ball_time - 0.5).max(0.0)),
                        }
                    });
                    ball_pub
                        .publish_with_source_time(&ball, at(ball_time))
                        .await?;
                }
                // Lower rate: everything else is held for the whole time in between.
                if now - last_slow_update >= 0.033 {
                    last_slow_update = now;
                    let image_time = now - 0.12;
                    if image_time > 0.0 {
                        localization_pub
                            .publish_with_source_time(
                                &ground_to_field_at(image_time),
                                at(image_time),
                            )
                            .await?;
                    }
                    let humanoid_time = now - 0.05;
                    if humanoid_time > 0.0 {
                        humanoids_pub
                            .publish_with_source_time(
                                &vec![
                                    Obstacle::robot(
                                        ground_to_field_at(humanoid_time).inverse()
                                            * humanoid_in_field,
                                        0.2,
                                        0.25,
                                    ),
                                    Obstacle::goal_post(
                                        ground_to_field_at(humanoid_time).inverse()
                                            * goal_post_in_field,
                                        0.1,
                                    ),
                                ],
                                at(humanoid_time),
                            )
                            .await?;
                    }
                    let image_time = now - 0.04;
                    if image_time > 0.0 {
                        visual_kick_pub
                            .publish_with_source_time(
                                &Some(BallPosition {
                                    position: ground_to_field_at(image_time).inverse()
                                        * ball_in_field,
                                    velocity: Vector2::zeros(),
                                    last_seen: at(image_time),
                                }),
                                at(image_time),
                            )
                            .await?;
                    }
                    let hypothesis_time = now - 0.09;
                    if hypothesis_time > 0.0 {
                        hypotheses_pub
                            .publish_with_source_time(
                                &vec![HypotheticalBallPosition {
                                    position: ground_to_field_at(hypothesis_time).inverse()
                                        * hypothesis_in_field,
                                    validity: 0.7,
                                }],
                                at(hypothesis_time),
                            )
                            .await?;
                    }
                    team_ball_pub
                        .publish(&Some(BallPosition {
                            position: team_ball_in_field,
                            velocity: Vector2::zeros(),
                            last_seen: at(now),
                        }))
                        .await?;
                    game_pub
                        .publish(&FilteredGameControllerState {
                            game_state: FilteredGameState::Ready,
                            opponent_game_state: FilteredGameState::Ready,
                            half: Half::First,
                            remaining_time_in_half: Duration::ZERO,
                            game_phase: GamePhase::Normal,
                            kicking_team: None,
                            penalties: Players::new(None),
                            remaining_number_of_messages: 0,
                            sub_state: None,
                            global_field_side: GlobalFieldSide::Home,
                            new_own_penalties_last_cycle: Default::default(),
                            new_opponent_penalties_last_cycle: Default::default(),
                        })
                        .await?;
                }
                tokio::time::sleep(Duration::from_millis(2)).await;
            }
            #[allow(unreachable_code)]
            Ok::<_, color_eyre::Report>(())
        })
    };

    let expected_close = |actual: Point2<Field>, expected: Point2<Field>, what: &str| {
        assert!(
            (actual - expected).norm() < TOLERANCE,
            "{what} is at {actual:?} instead of {expected:?}"
        );
    };
    let mut own_ball_checks = 0;
    let mut team_ball_checks = 0;
    let mut first_heading = None;
    let mut last_heading = 0.0;
    let checks = tokio::time::timeout(Duration::from_secs(30), async {
        while team_ball_checks < 10 {
            let blackboard = tokio::select! {
                blackboard = blackboards.recv() => blackboard?,
                failed = nodes.join_next() => panic!("a node exited: {failed:?}"),
            };
            let world_state = &blackboard.world_state;
            let Some(ground_to_field) = world_state.robot.ground_to_field else {
                continue;
            };
            first_heading.get_or_insert(ground_to_field.as_pose().angle());
            last_heading = ground_to_field.as_pose().angle();

            if let Some(rule_ball) = world_state.rule_ball {
                expected_close(
                    ground_to_field * rule_ball.ball_in_ground,
                    Point2::origin(),
                    "rule ball",
                );
            }
            for obstacle in &world_state.obstacles {
                let expected = match obstacle.kind {
                    types::obstacles::ObstacleKind::Robot => humanoid_in_field,
                    _ => goal_post_in_field,
                };
                expected_close(ground_to_field * obstacle.position, expected, "obstacle");
            }
            if let Some(ball) = blackboard.visual_kick_ball_position {
                expected_close(
                    ground_to_field * ball.position,
                    ball_in_field,
                    "visual ball",
                );
            }
            for hypothesis in &world_state.hypothetical_ball_positions {
                expected_close(
                    ground_to_field * hypothesis.position,
                    hypothesis_in_field,
                    "hypothetical ball",
                );
                assert_eq!(hypothesis.validity, 0.7);
            }
            let Some(ball) = world_state.ball else {
                continue;
            };
            match ball.source {
                BallSource::Own => {
                    // Fixed in the field, and where the robot sees it right now.
                    expected_close(ball.ball_in_field, ball_in_field, "field ball");
                    expected_close(
                        ground_to_field * ball.ball_in_ground,
                        ball_in_field,
                        "ground ball",
                    );
                    assert!(
                        (ground_to_field * ball.ball_in_ground_velocity - ball_velocity_in_field)
                            .norm()
                            < TOLERANCE
                    );
                    if world_state.obstacles.len() == 2
                        && blackboard.visual_kick_ball_position.is_some()
                        && world_state.rule_ball.is_some()
                        && world_state.hypothetical_ball_positions.len() == 1
                    {
                        own_ball_checks += 1;
                    }
                    if own_ball_checks == 30 {
                        // Lose the own ball; the team ball takes over.
                        own_ball.store(false, Ordering::Relaxed);
                    }
                }
                BallSource::Team => {
                    assert_eq!(ball.ball_in_field, team_ball_in_field);
                    expected_close(
                        ground_to_field * ball.ball_in_ground,
                        team_ball_in_field,
                        "ground team ball",
                    );
                    team_ball_checks += 1;
                }
                BallSource::Rule => panic!("ball_state must not be a rule ball"),
            }
        }
        Ok::<_, color_eyre::Report>(())
    })
    .await;
    fixture.abort();
    let _ = fixture.await;
    nodes.abort_all();
    while nodes.join_next().await.is_some() {}
    checks.expect("the robot should come to know everything")?;
    assert!(own_ball_checks >= 30);
    // Otherwise, there would be nothing to hold together.
    assert!(wrap(last_heading - first_heading.unwrap()).abs() > 0.2);
    Ok(())
}

fn wrap(angle: f32) -> f32 {
    angle.sin().atan2(angle.cos())
}
