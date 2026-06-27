use std::{collections::BTreeMap, time::SystemTime};

use bevy::prelude::*;
use coordinate_systems::{Ground, World};
use hsl_network_messages::PlayerNumber;
use linear_algebra::{Isometry2, Orientation2, Point2, Vector2};
use types::{
    field_dimensions::GlobalFieldSide,
    world_state::{BallState, RobotState, WorldState},
};

use crate::{
    behavior_tree_simulator::{
        SimulatedBall, SimulationConfig, SimulatorBall, SimulatorClock, SimulatorFallDownState,
        SimulatorGameState, SimulatorGroundToWorld, SimulatorHeadYaw, SimulatorPrimaryState,
        SimulatorReceivedHslMessages, SimulatorRobot, SimulatorRuleObstacles,
        SimulatorScenarioObstacles, SimulatorSuggestedSearchPosition,
    },
    communication::player_states_from_received_hsl_messages,
    coordinates::ground_to_field_from_world,
};

#[derive(Resource, Clone, Debug, Default)]
pub struct SimulatorWorldStates(pub BTreeMap<PlayerNumber, WorldState>);

pub(crate) fn build_world_states(
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
) {
    world_states.0.clear();
    let global_field_side = game_state.game_controller_state.global_field_side;
    let generated_obstacles = Vec::new();

    for (
        robot,
        ground_to_world,
        head_yaw,
        primary_state,
        fall_down_state,
        suggested_search_position,
    ) in &robots
    {
        let ground_to_field =
            ground_to_field_from_world(ground_to_world.ground_to_world, global_field_side);
        let perceived_ball = perceived_ball_from_pose(
            ball.state,
            ground_to_world.ground_to_world,
            global_field_side,
            clock.now,
            head_yaw.yaw,
            &config,
        );
        let obstacles = scenario_obstacles
            .obstacles
            .iter()
            .chain(&generated_obstacles)
            .copied()
            .map(|obstacle| obstacle.to_world_state_obstacle(ground_to_world.ground_to_world))
            .collect();

        world_states.0.insert(
            robot.player_number,
            WorldState {
                ball: perceived_ball,
                filtered_game_controller_state: game_state.filtered_game_controller_state.clone(),
                hypothetical_ball_positions: Vec::new(),
                now: clock.now.into(),
                obstacles,
                player_states: player_states_from_received_hsl_messages(
                    robot.player_number,
                    &received_hsl_messages,
                ),
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
                rule_obstacles: rule_obstacles.obstacles.clone(),
                fall_down_state: fall_down_state.fall_down_state,
                suggested_search_position: suggested_search_position.position,
            },
        );
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
    let distance = ball_in_ground.coords().norm();
    if distance > config.ball_visibility_range {
        return None;
    }

    struct Head;

    let head_to_ground = head_yaw.as_transform::<Head>();
    let ball_in_head = head_to_ground.inverse() * ball_in_ground;
    let angle = ball_in_head.coords().angle(&Vector2::x_axis());
    if angle.abs() > config.ball_visibility_angle / 2.0 {
        return None;
    }

    Some(ball.to_ball_state(ground_to_world, global_field_side, now))
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
    use hsl_network_messages::{HulkMessage, PlayerNumber};
    use linear_algebra::{Isometry2, Orientation2, Pose2, point, vector};
    use types::{
        ball_position::BallPosition,
        field_dimensions::{FieldDimensions, GlobalFieldSide, Side},
        messages::IncomingMessage,
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
            ball_visibility_angle: std::f32::consts::FRAC_PI_4,
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
    fn world_states_use_received_hsl_messages_for_teammate_state() {
        let mut app = App::new();
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
                    PlayerNumber::Four,
                    BTreeMap::from([(
                        PlayerNumber::Three,
                        SimulatorReceivedHslMessage {
                            message: teammate_message,
                            received_at,
                        },
                    )]),
                )]),
                player_states_by_receiver: BTreeMap::from([(
                    PlayerNumber::Four,
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

        let world_states = app.world().resource::<SimulatorWorldStates>();
        let receiver_world_state = world_states
            .0
            .get(&PlayerNumber::Four)
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

        let world_state = &app.world().resource::<SimulatorWorldStates>().0[&PlayerNumber::Four];
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
                    receiver: PlayerNumber::Four,
                    sender: PlayerNumber::Three,
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
            app.world().resource::<SimulatorWorldStates>().0[&PlayerNumber::Four].player_states
                [PlayerNumber::Three]
                .is_some()
        );

        app.world_mut().resource_mut::<SimulatorClock>().now += Duration::from_secs(1);
        app.update();

        let teammate_state = app.world().resource::<SimulatorWorldStates>().0[&PlayerNumber::Four]
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
}
