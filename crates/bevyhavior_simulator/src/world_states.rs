use std::{
    collections::BTreeMap,
    time::{Duration, SystemTime},
};

use bevy::prelude::*;
use coordinate_systems::{Field, Ground, World};
use hsl_network_messages::{GamePhase, SubState};
use linear_algebra::{Isometry2, Orientation2, Point2, Vector2, point};
use types::{
    ball_position::BallPosition,
    field_dimensions::{GlobalFieldSide, Side},
    filtered_game_controller_state::FilteredGameControllerState,
    players::Players,
    rule_obstacles::RuleObstacle,
    world_state::{BallState, PlayerState, RobotState, WorldState},
};

use crate::{
    behavior_tree_simulator::{
        SimulatedBall, SimulationConfig, SimulatorBall, SimulatorClock, SimulatorFallDownState,
        SimulatorGameState, SimulatorGroundToWorld, SimulatorHeadYaw, SimulatorObstacle,
        SimulatorPrimaryState, SimulatorReceivedHslMessages, SimulatorRobot, SimulatorRobotId,
        SimulatorRuleObstacles, SimulatorScenarioObstacles, SimulatorSuggestedSearchPosition,
    },
    communication::player_states_from_received_hsl_messages,
    coordinates::{ground_to_field_from_world, world_to_field_transform},
    game_controller::{filtered_game_controller_state_for_team, global_field_side_for_team},
};

#[derive(Resource, Clone, Debug, Default)]
pub struct SimulatorWorldStates(pub BTreeMap<SimulatorRobotId, WorldState>);

// Matches etc/parameters/base/team_ball_filter.json5.
const TEAM_BALL_MAXIMUM_AGE: Duration = Duration::from_millis(4500);

pub fn build_world_states(
    clock: Res<SimulatorClock>,
    ball: Res<SimulatorBall>,
    game_state: Res<SimulatorGameState>,
    received_hsl_messages: Res<SimulatorReceivedHslMessages>,
    rule_obstacles: Res<SimulatorRuleObstacles>,
    scenario_obstacles: Res<SimulatorScenarioObstacles>,
    config: Res<SimulationConfig>,
    robots: Query<(
        &SimulatorRobot,
        &SimulatorGroundToWorld,
        &SimulatorHeadYaw,
        &SimulatorPrimaryState,
        &SimulatorFallDownState,
        &SimulatorSuggestedSearchPosition,
    )>,
    mut world_states: ResMut<SimulatorWorldStates>,
    mut last_ball_field_sides: Local<BTreeMap<SimulatorRobotId, Side>>,
) {
    world_states.0.clear();
    let canonical_global_field_side = game_state.game_controller_state.global_field_side;
    let robot_poses = robots
        .iter()
        .map(|(robot, ground_to_world, _, _, _, _)| (robot.id(), ground_to_world.ground_to_world))
        .collect::<Vec<_>>();

    for (
        robot,
        ground_to_world,
        head_yaw,
        primary_state,
        fall_down_state,
        suggested_search_position,
    ) in &robots
    {
        let robot_id = robot.id();
        let global_field_side =
            global_field_side_for_team(&game_state.game_controller_state, robot.team);
        let ground_to_field =
            ground_to_field_from_world(ground_to_world.ground_to_world, global_field_side);
        let filtered_game_controller_state =
            filtered_game_controller_state_for_team(&game_state.game_controller_state, robot.team);
        let player_states =
            player_states_from_received_hsl_messages(robot_id, &received_hsl_messages);
        let team_ball = team_ball_from_player_states(
            &player_states,
            &filtered_game_controller_state,
            clock.now,
        );
        let perceived_ball = perceived_ball_from_pose(
            ball.state,
            ground_to_world.ground_to_world,
            global_field_side,
            clock.now,
            head_yaw.yaw,
            &config,
        );
        let generated_obstacles = perceived_robot_obstacles_from_pose(
            robot_id,
            ground_to_world.ground_to_world,
            head_yaw.yaw,
            &robot_poses,
            &config,
        );
        let obstacles = scenario_obstacles
            .obstacles
            .iter()
            .chain(generated_obstacles.iter())
            .copied()
            .map(|obstacle| obstacle.to_world_state_obstacle(ground_to_world.ground_to_world))
            .collect();
        let rule_obstacles = rule_obstacles
            .obstacles
            .iter()
            .copied()
            .map(|obstacle| {
                rule_obstacle_for_team(obstacle, canonical_global_field_side, global_field_side)
            })
            .collect();

        world_states.0.insert(
            robot_id,
            WorldState {
                ball: compose_ball_state(
                    perceived_ball,
                    team_ball,
                    ground_to_field,
                    last_ball_field_sides.entry(robot_id).or_insert(Side::Left),
                ),
                filtered_game_controller_state: Some(filtered_game_controller_state),
                hypothetical_ball_positions: Vec::new(),
                now: clock.now.into(),
                obstacles,
                player_states,
                position_of_interest: Point2::origin(),
                robot: RobotState {
                    ground_to_field: Some(ground_to_field),
                    player_number: robot.player_number,
                    primary_state: primary_state.primary_state,
                },
                rule_ball: ball.state.map(|ball| {
                    ball.to_ball_state(
                        ground_to_world.ground_to_world,
                        global_field_side,
                        clock.now,
                    )
                }),
                rule_obstacles,
                fall_down_state: fall_down_state.fall_down_state,
                suggested_search_position: suggested_search_position.position,
            },
        );
    }
}

fn team_ball_from_player_states(
    player_states: &Players<Option<PlayerState>>,
    game_controller_state: &FilteredGameControllerState,
    now: SystemTime,
) -> Option<BallPosition<Field>> {
    // Mirror team_ball_filter: suppress team balls during penalties and use the newest report.
    if matches!(
        game_controller_state.game_phase,
        GamePhase::PenaltyShootout { .. }
    ) || game_controller_state.sub_state == Some(SubState::PenaltyKick)
    {
        return None;
    }

    player_states
        .iter()
        .filter_map(|(_, player)| player.and_then(|player| player.ball_position))
        .max_by_key(|ball| ball.last_seen)
        .filter(|ball| {
            ball.age_at(now.into())
                .is_some_and(|age| age < TEAM_BALL_MAXIMUM_AGE)
        })
}

fn compose_ball_state(
    perceived_ball: Option<BallState>,
    team_ball: Option<BallPosition<Field>>,
    ground_to_field: Isometry2<Ground, Field>,
    last_ball_field_side: &mut Side,
) -> Option<BallState> {
    // Mirror ball_state_composer: prefer local vision and preserve the team's observation time.
    let mut ball = perceived_ball.or_else(|| {
        let team_ball = team_ball?;
        let ball_in_ground = ground_to_field.inverse() * team_ball;
        Some(BallState {
            ball_in_ground: ball_in_ground.position,
            ball_in_field: team_ball.position,
            ball_in_ground_velocity: ball_in_ground.velocity,
            last_seen_ball: team_ball.last_seen.to_wallclock(),
            field_side: *last_ball_field_side,
        })
    })?;

    // Same 0.2-wide hysteresis around the center line as ball_state_composer.
    let y = ball.ball_in_field.y();
    if !(-0.1..=0.1).contains(&y) {
        *last_ball_field_side = if y > 0.1 { Side::Left } else { Side::Right };
    }
    ball.field_side = *last_ball_field_side;
    Some(ball)
}

fn rule_obstacle_for_team(
    obstacle: RuleObstacle,
    canonical_global_field_side: GlobalFieldSide,
    robot_global_field_side: GlobalFieldSide,
) -> RuleObstacle {
    let canonical_field_to_world = world_to_field_transform(canonical_global_field_side).inverse();
    let world_to_robot_field = world_to_field_transform(robot_global_field_side);

    match obstacle {
        RuleObstacle::Circle(circle) => RuleObstacle::Circle(geometry::circle::Circle {
            center: world_to_robot_field * (canonical_field_to_world * circle.center),
            radius: circle.radius,
        }),
        RuleObstacle::Rectangle(rectangle) => {
            let first_corner = world_to_robot_field * (canonical_field_to_world * rectangle.min);
            let second_corner = world_to_robot_field * (canonical_field_to_world * rectangle.max);
            RuleObstacle::Rectangle(geometry::rectangle::Rectangle {
                min: point![
                    first_corner.x().min(second_corner.x()),
                    first_corner.y().min(second_corner.y())
                ],
                max: point![
                    first_corner.x().max(second_corner.x()),
                    first_corner.y().max(second_corner.y())
                ],
            })
        }
    }
}

fn perceived_ball_from_pose(
    ball: Option<SimulatedBall>,
    ground_to_world: Isometry2<Ground, World>,
    global_field_side: GlobalFieldSide,
    now: SystemTime,
    head_yaw: Orientation2<Ground>,
    config: &SimulationConfig,
) -> Option<BallState> {
    let ball = ball?;
    let ball_in_ground = ground_to_world.inverse() * ball.position;
    if !is_visible_from_head(ball_in_ground, head_yaw, config) {
        return None;
    }

    Some(ball.to_ball_state(ground_to_world, global_field_side, now))
}

fn perceived_robot_obstacles_from_pose(
    receiver_id: SimulatorRobotId,
    receiver_ground_to_world: Isometry2<Ground, World>,
    receiver_head_yaw: Orientation2<Ground>,
    robot_poses: &[(SimulatorRobotId, Isometry2<Ground, World>)],
    config: &SimulationConfig,
) -> Vec<SimulatorObstacle> {
    robot_poses
        .iter()
        .filter_map(|(robot_id, ground_to_world)| {
            if *robot_id == receiver_id {
                return None;
            }

            let position = ground_to_world * Point2::origin();
            let position_in_receiver_ground = receiver_ground_to_world.inverse() * position;
            is_visible_from_head(position_in_receiver_ground, receiver_head_yaw, config).then(
                || SimulatorObstacle::robot(position, config.robot_radius, config.robot_radius),
            )
        })
        .collect()
}

fn is_visible_from_head(
    position_in_ground: Point2<Ground>,
    head_yaw: Orientation2<Ground>,
    config: &SimulationConfig,
) -> bool {
    let distance = position_in_ground.coords().norm();
    if distance > config.ball_visibility_range {
        return false;
    }

    struct Head;

    let head_to_ground = head_yaw.as_transform::<Head>();
    let position_in_head = head_to_ground.inverse() * position_in_ground;
    let angle = position_in_head.coords().angle(&Vector2::x_axis());
    angle.abs() <= config.visibility_field_of_view / 2.0
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeMap,
        f32::consts::{FRAC_PI_2, PI},
        time::Duration,
        time::SystemTime,
    };

    use approx::assert_relative_eq;
    use hsl_network_messages::{HulkMessage, PlayerNumber, Team};
    use linear_algebra::{Isometry2, Orientation2, Pose2, point, vector};
    use types::{
        ball_position::BallPosition,
        field_dimensions::{FieldDimensions, GlobalFieldSide, Side},
        messages::IncomingMessage,
        obstacles::ObstacleKind,
        players::Players,
        primary_state::PrimaryState,
        world_state::PlayerState,
    };

    use super::*;
    use crate::{
        behavior_tree_simulator::{
            DEFAULT_TICK_DURATION, SimulatedBall, SimulationConfig, SimulatorBall, SimulatorClock,
            SimulatorFallDownState, SimulatorGameState, SimulatorGroundToWorld, SimulatorHeadYaw,
            SimulatorIncomingMessage, SimulatorIncomingMessages, SimulatorPrimaryState,
            SimulatorReceivedHslMessage, SimulatorReceivedHslMessages, SimulatorRobot,
            SimulatorRuleObstacles, SimulatorScenarioObstacles, SimulatorSuggestedSearchPosition,
        },
        communication::apply_incoming_hsl_messages,
    };

    fn hsl_state_message(player_number: PlayerNumber, x: f32, y: f32) -> HulkMessage {
        HulkMessage::State(hsl_network_messages::StateMessage {
            player_number,
            pose: Pose2::new(point![x, y], 0.0),
            ball_position: Some(hsl_network_messages::BallPosition {
                age: Duration::from_millis(500),
                position: point![x + 1.0, y],
            }),
        })
    }

    fn robot_id(player_number: PlayerNumber) -> SimulatorRobotId {
        SimulatorRobotId::new(Team::Hulks, player_number)
    }

    fn ball_at(x: f32, y: f32) -> Option<SimulatedBall> {
        Some(SimulatedBall {
            position: point![x, y],
            velocity: vector![0.0, 0.0],
            field_side: Side::Left,
        })
    }

    #[test]
    fn ball_visibility_uses_head_yaw() {
        let config = SimulationConfig {
            visibility_field_of_view: std::f32::consts::FRAC_PI_4,
            ..Default::default()
        };

        assert!(
            perceived_ball_from_pose(
                ball_at(0.0, 1.0),
                Isometry2::identity(),
                GlobalFieldSide::Home,
                SystemTime::UNIX_EPOCH,
                Orientation2::new(FRAC_PI_2),
                &config,
            )
            .is_some()
        );
        assert!(
            perceived_ball_from_pose(
                ball_at(1.0, 0.0),
                Isometry2::identity(),
                GlobalFieldSide::Home,
                SystemTime::UNIX_EPOCH,
                Orientation2::new(FRAC_PI_2),
                &config,
            )
            .is_none()
        );
    }

    #[test]
    fn team_ball_selection_uses_newest_observation_and_rejects_penalties() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(10);
        let mut game_state = SimulatorGameState::default()
            .filtered_game_controller_state
            .expect("filtered game state should exist");
        let mut player_states = Players::default();
        for (player_number, age, position) in [
            (PlayerNumber::One, Duration::from_secs(2), point![1.0, 0.0]),
            (PlayerNumber::Five, Duration::from_secs(1), point![3.0, 0.0]),
        ] {
            player_states[player_number] = Some(PlayerState {
                pose: Pose2::default(),
                ball_position: Some(BallPosition::from_network_ball(
                    hsl_network_messages::BallPosition { age, position },
                    now.into(),
                )),
            });
        }

        let ball = team_ball_from_player_states(&player_states, &game_state, now)
            .expect("freshest team ball should be selected");
        assert_eq!(ball.position, point![3.0, 0.0]);
        assert_eq!(ball.last_seen.to_wallclock(), now - Duration::from_secs(1));

        for (game_phase, sub_state) in [
            (
                GamePhase::PenaltyShootout {
                    kicking_team: Team::Hulks,
                },
                None,
            ),
            (GamePhase::Normal, Some(SubState::PenaltyKick)),
        ] {
            game_state.game_phase = game_phase;
            game_state.sub_state = sub_state;
            assert!(team_ball_from_player_states(&player_states, &game_state, now).is_none());
        }

        game_state.game_phase = GamePhase::Normal;
        game_state.sub_state = None;
        assert!(
            team_ball_from_player_states(&player_states, &game_state, now - Duration::from_secs(2))
                .is_none(),
            "a report from the future must be rejected"
        );
    }

    #[test]
    fn world_states_generate_visible_robot_obstacles() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(SimulatorClock {
                now: SystemTime::UNIX_EPOCH,
                tick_duration: DEFAULT_TICK_DURATION,
            })
            .insert_resource(crate::behavior_tree_simulator::SimulatorFieldDimensions(
                FieldDimensions::SPL_2025,
            ))
            .insert_resource(SimulatorBall::default())
            .insert_resource(SimulatorGameState::default())
            .insert_resource(SimulatorReceivedHslMessages::default())
            .insert_resource(SimulatorRuleObstacles::default())
            .insert_resource(SimulatorScenarioObstacles::default())
            .insert_resource(SimulationConfig {
                visibility_field_of_view: FRAC_PI_2,
                ball_visibility_range: 2.0,
                robot_radius: 0.25,
                ..Default::default()
            })
            .insert_resource(SimulatorWorldStates::default())
            .add_systems(Update, build_world_states);

        for (player_number, pose) in [
            (PlayerNumber::One, Isometry2::identity()),
            (
                PlayerNumber::Two,
                Isometry2::from_parts(vector![1.0, 0.0], 0.0),
            ),
            (
                PlayerNumber::Three,
                Isometry2::from_parts(vector![0.0, 1.0], 0.0),
            ),
        ] {
            app.world_mut().spawn((
                SimulatorRobot {
                    team: Team::Hulks,
                    player_number,
                },
                SimulatorGroundToWorld {
                    ground_to_world: pose,
                },
                SimulatorHeadYaw::default(),
                SimulatorPrimaryState {
                    primary_state: PrimaryState::Playing,
                },
                SimulatorFallDownState::default(),
                SimulatorSuggestedSearchPosition::default(),
            ));
        }

        app.update();

        let world_state =
            &app.world().resource::<SimulatorWorldStates>().0[&robot_id(PlayerNumber::One)];
        assert_eq!(world_state.obstacles.len(), 1);
        let obstacle = world_state.obstacles[0];
        assert_eq!(obstacle.kind, ObstacleKind::Robot);
        assert_relative_eq!(obstacle.position.x(), 1.0, epsilon = 0.0001);
        assert_relative_eq!(obstacle.position.y(), 0.0, epsilon = 0.0001);
        assert_relative_eq!(obstacle.radius_at_foot_height, 0.25, epsilon = 0.0001);
        assert_relative_eq!(obstacle.radius_at_hip_height, 0.25, epsilon = 0.0001);
    }

    #[test]
    fn world_states_use_received_hsl_messages_for_teammate_state_and_ball() {
        let mut app = App::new();
        let receiver_id = SimulatorRobotId::new(Team::Opponent, PlayerNumber::Four);
        let sender_id = SimulatorRobotId::new(Team::Opponent, PlayerNumber::Three);
        let received_at = SystemTime::UNIX_EPOCH + Duration::from_secs(2);
        let teammate_message = hsl_state_message(PlayerNumber::Three, 1.0, 0.5);
        app.add_plugins(MinimalPlugins)
            .insert_resource(SimulatorClock {
                now: SystemTime::UNIX_EPOCH + Duration::from_secs(3),
                tick_duration: DEFAULT_TICK_DURATION,
            })
            .insert_resource(crate::behavior_tree_simulator::SimulatorFieldDimensions(
                FieldDimensions::SPL_2025,
            ))
            .insert_resource(SimulatorBall::default())
            .insert_resource(SimulatorGameState::default())
            .insert_resource(SimulatorReceivedHslMessages {
                messages_by_receiver: BTreeMap::from([(
                    receiver_id,
                    BTreeMap::from([(
                        sender_id,
                        SimulatorReceivedHslMessage {
                            message: teammate_message,
                            received_at,
                        },
                    )]),
                )]),
                player_states_by_receiver: BTreeMap::from([(
                    receiver_id,
                    Players {
                        three: Some(PlayerState {
                            pose: Pose2::new(point![1.0, 0.5], 0.0),
                            ball_position: Some(BallPosition::from_network_ball(
                                hsl_network_messages::BallPosition {
                                    age: Duration::from_millis(500),
                                    position: point![2.0, 0.5],
                                },
                                ros_z::time::Time::from_wallclock(received_at),
                            )),
                        }),
                        ..Default::default()
                    },
                )]),
            })
            .insert_resource(SimulatorRuleObstacles::default())
            .insert_resource(SimulatorScenarioObstacles::default())
            .insert_resource(SimulationConfig::default())
            .insert_resource(SimulatorWorldStates::default())
            .add_systems(Update, build_world_states);
        app.world_mut().spawn((
            SimulatorRobot {
                team: Team::Opponent,
                player_number: PlayerNumber::Four,
            },
            SimulatorGroundToWorld {
                ground_to_world: Isometry2::from_parts(vector![1.0, -0.5], FRAC_PI_2),
            },
            SimulatorHeadYaw::default(),
            SimulatorPrimaryState {
                primary_state: PrimaryState::Playing,
            },
            SimulatorFallDownState::default(),
            SimulatorSuggestedSearchPosition::default(),
        ));

        app.update();

        let world_states = app.world().resource::<SimulatorWorldStates>();
        let receiver_world_state = world_states
            .0
            .get(&receiver_id)
            .expect("receiver world state should exist");
        let teammate_state = receiver_world_state.player_states[PlayerNumber::Three]
            .expect("teammate state should come from HSL message");
        assert_eq!(teammate_state.pose.position(), point![1.0, 0.5]);
        assert_eq!(
            teammate_state
                .ball_position
                .expect("teammate ball should come from HSL message")
                .position,
            point![2.0, 0.5]
        );
        assert!(receiver_world_state.player_states[PlayerNumber::Four].is_none());
        let team_ball = receiver_world_state
            .ball
            .expect("received team ball should be available without local vision");
        assert_eq!(team_ball.ball_in_field, point![2.0, 0.5]);
        assert_relative_eq!(team_ball.ball_in_ground.x(), 0.0, epsilon = 0.0001);
        assert_relative_eq!(team_ball.ball_in_ground.y(), 3.0, epsilon = 0.0001);
        assert_eq!(
            team_ball.last_seen_ball,
            received_at - Duration::from_millis(500)
        );

        app.world_mut().resource_mut::<SimulatorBall>().state = ball_at(1.0, 0.5);
        app.update();
        let local_ball = app.world().resource::<SimulatorWorldStates>().0[&receiver_id]
            .ball
            .expect("local vision should take precedence over the team ball");
        assert_relative_eq!(local_ball.ball_in_field.x(), -1.0, epsilon = 0.0001);
        assert_relative_eq!(local_ball.ball_in_field.y(), -0.5, epsilon = 0.0001);

        app.world_mut().resource_mut::<SimulatorBall>().state = None;
        app.world_mut().resource_mut::<SimulatorClock>().now =
            team_ball.last_seen_ball + TEAM_BALL_MAXIMUM_AGE - Duration::from_millis(1);
        app.update();
        assert_eq!(
            app.world().resource::<SimulatorWorldStates>().0[&receiver_id].ball,
            Some(team_ball),
        );

        app.world_mut().resource_mut::<SimulatorClock>().now += Duration::from_millis(1);
        app.update();
        assert!(
            app.world().resource::<SimulatorWorldStates>().0[&receiver_id]
                .ball
                .is_none(),
            "team ball should expire even though the received player state persists"
        );
    }

    #[test]
    fn world_states_flip_pose_and_ball_for_away_side() {
        let mut app = App::new();
        let mut game_state = SimulatorGameState::default();
        game_state.game_controller_state.global_field_side = GlobalFieldSide::Away;
        game_state.sync_filtered_game_controller_state();

        app.add_plugins(MinimalPlugins)
            .insert_resource(SimulatorClock {
                now: SystemTime::UNIX_EPOCH + Duration::from_secs(1),
                tick_duration: DEFAULT_TICK_DURATION,
            })
            .insert_resource(crate::behavior_tree_simulator::SimulatorFieldDimensions(
                FieldDimensions::SPL_2025,
            ))
            .insert_resource(SimulatorBall {
                state: Some(SimulatedBall {
                    position: point![1.0, 0.0],
                    velocity: vector![0.0, 0.0],
                    field_side: Side::Left,
                }),
                last_touched_by: None,
            })
            .insert_resource(game_state)
            .insert_resource(SimulatorReceivedHslMessages::default())
            .insert_resource(SimulatorRuleObstacles::default())
            .insert_resource(SimulatorScenarioObstacles::default())
            .insert_resource(SimulationConfig::default())
            .insert_resource(SimulatorWorldStates::default())
            .add_systems(Update, build_world_states);
        app.world_mut().spawn((
            SimulatorRobot {
                team: Team::Hulks,
                player_number: PlayerNumber::Four,
            },
            SimulatorGroundToWorld {
                ground_to_world: Isometry2::identity(),
            },
            SimulatorHeadYaw::default(),
            SimulatorPrimaryState {
                primary_state: PrimaryState::Playing,
            },
            SimulatorFallDownState::default(),
            SimulatorSuggestedSearchPosition::default(),
        ));

        app.update();

        let world_state =
            &app.world().resource::<SimulatorWorldStates>().0[&robot_id(PlayerNumber::Four)];
        let ground_to_field = world_state
            .robot
            .ground_to_field
            .expect("ground_to_field should be provided to behavior");
        assert_relative_eq!(
            ground_to_field.orientation().angle().abs(),
            PI,
            epsilon = 0.0001
        );
        let ball = world_state.ball.expect("ball should be visible");
        assert_relative_eq!(ball.ball_in_field.x(), -1.0, epsilon = 0.0001);
        assert_relative_eq!(ball.ball_in_field.y(), 0.0, epsilon = 0.0001);
        assert_eq!(ball.ball_in_ground, point![1.0, 0.0]);
    }

    #[test]
    fn player_state_persists_without_new_hsl_message() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(SimulatorClock {
                now: SystemTime::UNIX_EPOCH + Duration::from_secs(3),
                tick_duration: DEFAULT_TICK_DURATION,
            })
            .insert_resource(crate::behavior_tree_simulator::SimulatorFieldDimensions(
                FieldDimensions::SPL_2025,
            ))
            .insert_resource(SimulatorBall::default())
            .insert_resource(SimulatorGameState::default())
            .insert_resource(SimulatorIncomingMessages {
                messages: vec![SimulatorIncomingMessage {
                    receiver: robot_id(PlayerNumber::Four),
                    sender: robot_id(PlayerNumber::Three),
                    message: IncomingMessage::Hsl(hsl_state_message(PlayerNumber::Three, 1.0, 0.5)),
                    received_at: SystemTime::UNIX_EPOCH + Duration::from_secs(2),
                }],
            })
            .insert_resource(SimulatorReceivedHslMessages::default())
            .insert_resource(SimulatorRuleObstacles::default())
            .insert_resource(SimulatorScenarioObstacles::default())
            .insert_resource(SimulationConfig::default())
            .insert_resource(SimulatorWorldStates::default())
            .add_systems(
                Update,
                (apply_incoming_hsl_messages, build_world_states).chain(),
            );
        app.world_mut().spawn((
            SimulatorRobot {
                team: Team::Hulks,
                player_number: PlayerNumber::Four,
            },
            SimulatorGroundToWorld {
                ground_to_world: Isometry2::identity(),
            },
            SimulatorHeadYaw::default(),
            SimulatorPrimaryState {
                primary_state: PrimaryState::Playing,
            },
            SimulatorFallDownState::default(),
            SimulatorSuggestedSearchPosition::default(),
        ));

        app.update();
        assert!(
            app.world().resource::<SimulatorWorldStates>().0[&robot_id(PlayerNumber::Four)]
                .player_states[PlayerNumber::Three]
                .is_some()
        );

        app.world_mut().resource_mut::<SimulatorClock>().now += Duration::from_secs(1);
        app.update();

        let teammate_state = app.world().resource::<SimulatorWorldStates>().0
            [&robot_id(PlayerNumber::Four)]
            .player_states[PlayerNumber::Three]
            .expect("teammate state should persist without new HSL messages");
        assert_eq!(teammate_state.pose.position(), point![1.0, 0.5]);
        assert!(
            app.world()
                .resource::<SimulatorIncomingMessages>()
                .messages
                .is_empty()
        );
    }

    #[test]
    fn world_states_keep_same_player_number_on_both_teams() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(SimulatorClock {
                now: SystemTime::UNIX_EPOCH + Duration::from_secs(1),
                tick_duration: DEFAULT_TICK_DURATION,
            })
            .insert_resource(crate::behavior_tree_simulator::SimulatorFieldDimensions(
                FieldDimensions::SPL_2025,
            ))
            .insert_resource(SimulatorBall::default())
            .insert_resource(SimulatorGameState::default())
            .insert_resource(SimulatorReceivedHslMessages::default())
            .insert_resource(SimulatorRuleObstacles::default())
            .insert_resource(SimulatorScenarioObstacles::default())
            .insert_resource(SimulationConfig::default())
            .insert_resource(SimulatorWorldStates::default())
            .add_systems(Update, build_world_states);

        for team in [Team::Hulks, Team::Opponent] {
            app.world_mut().spawn((
                SimulatorRobot {
                    team,
                    player_number: PlayerNumber::Three,
                },
                SimulatorGroundToWorld {
                    ground_to_world: Isometry2::identity(),
                },
                SimulatorHeadYaw::default(),
                SimulatorPrimaryState {
                    primary_state: PrimaryState::Playing,
                },
                SimulatorFallDownState::default(),
                SimulatorSuggestedSearchPosition::default(),
            ));
        }

        app.update();

        let world_states = &app.world().resource::<SimulatorWorldStates>().0;
        let hulks_state = &world_states[&SimulatorRobotId::new(Team::Hulks, PlayerNumber::Three)];
        let opponent_state =
            &world_states[&SimulatorRobotId::new(Team::Opponent, PlayerNumber::Three)];
        assert_eq!(world_states.len(), 2);
        assert_relative_eq!(
            hulks_state
                .robot
                .ground_to_field
                .expect("ground_to_field should exist")
                .orientation()
                .angle(),
            0.0,
            epsilon = 0.0001
        );
        assert_relative_eq!(
            opponent_state
                .robot
                .ground_to_field
                .expect("ground_to_field should exist")
                .orientation()
                .angle()
                .abs(),
            PI,
            epsilon = 0.0001
        );
    }
}
