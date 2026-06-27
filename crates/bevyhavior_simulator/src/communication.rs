use std::{collections::BTreeMap, time::SystemTime};

use bevy::prelude::*;
use hsl_network_messages::{HulkMessage, Team};
use serde::Serialize;
use types::{
    ball_position::BallPosition,
    messages::{IncomingMessage, OutgoingMessage},
    parameters::HslNetworkParameters,
    players::Players,
    world_state::PlayerState,
};

use crate::behavior_tree_simulator::{
    SimulationConfig, SimulatorClock, SimulatorGameState, SimulatorRobot, SimulatorRobotBehavior,
    SimulatorRobotFrames, SimulatorRobotId, SimulatorWorldStates,
};

#[derive(Resource, Clone, Debug, Default)]
pub struct SimulatorIncomingMessages {
    pub messages: Vec<SimulatorIncomingMessage>,
}

#[derive(Resource, Clone, Debug, Default)]
pub struct SimulatorOutgoingMessages {
    pub messages: Vec<SimulatorMessage>,
}

#[derive(Resource, Clone, Debug, Default)]
pub struct SimulatorReceivedHslMessages {
    pub messages_by_receiver:
        BTreeMap<SimulatorRobotId, BTreeMap<SimulatorRobotId, SimulatorReceivedHslMessage>>,
    pub player_states_by_receiver: BTreeMap<SimulatorRobotId, Players<Option<PlayerState>>>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SimulatorMessage {
    pub sender: SimulatorRobotId,
    pub message: OutgoingMessage,
}

#[derive(Clone, Debug, Serialize)]
pub struct SimulatorIncomingMessage {
    pub receiver: SimulatorRobotId,
    pub sender: SimulatorRobotId,
    pub message: IncomingMessage,
    pub received_at: SystemTime,
}

#[derive(Clone, Debug)]
pub struct SimulatorReceivedHslMessage {
    pub message: HulkMessage,
    pub received_at: SystemTime,
}

#[derive(Resource, Clone, Debug)]
pub struct SimulatorHslNetworkParameters(pub HslNetworkParameters);

pub(crate) fn plan_communication(
    config: Res<SimulationConfig>,
    hsl_network_parameters: Res<SimulatorHslNetworkParameters>,
    world_states: Res<SimulatorWorldStates>,
    mut robot_frames: ResMut<SimulatorRobotFrames>,
    mut outgoing_messages: ResMut<SimulatorOutgoingMessages>,
    mut robots: Query<(&SimulatorRobot, &mut SimulatorRobotBehavior)>,
) {
    outgoing_messages.messages.clear();

    for (robot, mut behavior) in &mut robots {
        let robot_id = robot.id();
        let Some(world_state) = world_states.0.get(&robot_id) else {
            continue;
        };

        let outgoing_robot_messages = behavior.plan_communication(
            world_state.clone(),
            hsl_network_parameters.0.clone(),
            config.game_controller_address,
        );

        if let Some(frame) = robot_frames.0.get_mut(&robot_id) {
            frame.outgoing_messages = outgoing_robot_messages.clone();
        }

        outgoing_messages
            .messages
            .extend(
                outgoing_robot_messages
                    .into_iter()
                    .map(|message| SimulatorMessage {
                        sender: robot_id,
                        message,
                    }),
            );
    }
}

pub(crate) fn apply_incoming_hsl_messages(
    mut incoming_messages: ResMut<SimulatorIncomingMessages>,
    mut received_hsl_messages: ResMut<SimulatorReceivedHslMessages>,
) {
    for incoming_message in incoming_messages.messages.drain(..) {
        let IncomingMessage::Hsl(message) = incoming_message.message else {
            continue;
        };

        let HulkMessage::State(state_message) = message;
        let player_state = PlayerState {
            pose: state_message.pose,
            ball_position: state_message.ball_position.map(|ball| {
                BallPosition::from_network_ball(
                    ball,
                    ros_z::time::Time::from_wallclock(incoming_message.received_at),
                )
            }),
        };
        received_hsl_messages
            .player_states_by_receiver
            .entry(incoming_message.receiver)
            .or_default()[state_message.player_number] = Some(player_state);

        received_hsl_messages
            .messages_by_receiver
            .entry(incoming_message.receiver)
            .or_default()
            .insert(
                incoming_message.sender,
                SimulatorReceivedHslMessage {
                    message,
                    received_at: incoming_message.received_at,
                },
            );
    }
}

pub(crate) fn route_outgoing_communication(
    clock: Res<SimulatorClock>,
    outgoing_messages: Res<SimulatorOutgoingMessages>,
    mut incoming_messages: ResMut<SimulatorIncomingMessages>,
    mut game_state: ResMut<SimulatorGameState>,
    robots: Query<&SimulatorRobot>,
) {
    incoming_messages.messages.clear();

    for outgoing_message in &outgoing_messages.messages {
        let OutgoingMessage::Hsl(message) = outgoing_message.message.clone() else {
            continue;
        };

        let remaining_amount_of_messages = match outgoing_message.sender.team {
            Team::Hulks => {
                &mut game_state
                    .game_controller_state
                    .hulks_team
                    .remaining_amount_of_messages
            }
            Team::Opponent => {
                &mut game_state
                    .game_controller_state
                    .opponent_team
                    .remaining_amount_of_messages
            }
        };
        if *remaining_amount_of_messages == 0 {
            continue;
        }
        *remaining_amount_of_messages = remaining_amount_of_messages.saturating_sub(1);

        for robot in &robots {
            if robot.team != outgoing_message.sender.team || robot.id() == outgoing_message.sender {
                continue;
            }

            incoming_messages.messages.push(SimulatorIncomingMessage {
                receiver: robot.id(),
                sender: outgoing_message.sender,
                message: IncomingMessage::Hsl(message),
                received_at: clock.now,
            });
        }
    }

    game_state.sync_filtered_game_controller_state();
}

pub(crate) fn player_states_from_received_hsl_messages(
    receiver: SimulatorRobotId,
    received_hsl_messages: &SimulatorReceivedHslMessages,
) -> Players<Option<PlayerState>> {
    received_hsl_messages
        .player_states_by_receiver
        .get(&receiver)
        .copied()
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime};

    use hsl_network_messages::{HulkMessage, PlayerNumber, Team};
    use linear_algebra::{Pose2, point};
    use types::messages::{IncomingMessage, OutgoingMessage};

    use super::*;
    use crate::behavior_tree_simulator::{
        DEFAULT_TICK_DURATION, SimulatorClock, SimulatorGameState, SimulatorRobot,
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

    fn opponent_robot_id(player_number: PlayerNumber) -> SimulatorRobotId {
        SimulatorRobotId::new(Team::Opponent, player_number)
    }

    fn game_state_with_message_budget(remaining_amount_of_messages: u16) -> SimulatorGameState {
        let mut game_state = SimulatorGameState::default();
        game_state
            .game_controller_state
            .hulks_team
            .remaining_amount_of_messages = remaining_amount_of_messages;
        game_state.sync_filtered_game_controller_state();
        game_state
    }

    fn route_test_app(remaining_amount_of_messages: u16) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(SimulatorClock {
                now: SystemTime::UNIX_EPOCH + Duration::from_secs(1),
                tick_duration: DEFAULT_TICK_DURATION,
            })
            .insert_resource(SimulatorOutgoingMessages::default())
            .insert_resource(SimulatorIncomingMessages::default())
            .insert_resource(game_state_with_message_budget(remaining_amount_of_messages))
            .add_systems(Update, route_outgoing_communication);

        for player_number in [PlayerNumber::Three, PlayerNumber::Four, PlayerNumber::Five] {
            app.world_mut().spawn(SimulatorRobot {
                team: Team::Hulks,
                player_number,
            });
        }

        app
    }

    #[test]
    fn hsl_broadcast_routes_to_teammates_and_decrements_budget_once() {
        let mut app = route_test_app(5);
        app.world_mut()
            .resource_mut::<SimulatorOutgoingMessages>()
            .messages
            .push(SimulatorMessage {
                sender: robot_id(PlayerNumber::Three),
                message: OutgoingMessage::Hsl(hsl_state_message(PlayerNumber::Three, 1.0, 0.0)),
            });

        app.update();

        let incoming_messages = &app.world().resource::<SimulatorIncomingMessages>().messages;
        assert_eq!(incoming_messages.len(), 2);
        assert!(
            incoming_messages
                .iter()
                .all(|message| message.sender == robot_id(PlayerNumber::Three))
        );
        assert!(
            incoming_messages
                .iter()
                .all(|message| message.receiver != robot_id(PlayerNumber::Three))
        );
        assert!(
            incoming_messages
                .iter()
                .any(|message| message.receiver == robot_id(PlayerNumber::Four))
        );
        assert!(
            incoming_messages
                .iter()
                .any(|message| message.receiver == robot_id(PlayerNumber::Five))
        );

        let game_state = app.world().resource::<SimulatorGameState>();
        assert_eq!(
            game_state
                .game_controller_state
                .hulks_team
                .remaining_amount_of_messages,
            4
        );
        assert_eq!(
            game_state
                .filtered_game_controller_state
                .as_ref()
                .expect("filtered game state should exist")
                .remaining_number_of_messages,
            4
        );
    }

    #[test]
    fn hsl_broadcast_routes_only_to_same_team_and_decrements_that_budget() {
        let mut app = route_test_app(5);
        app.world_mut()
            .resource_mut::<SimulatorGameState>()
            .game_controller_state
            .opponent_team
            .remaining_amount_of_messages = 3;
        for player_number in [PlayerNumber::Three, PlayerNumber::Four] {
            app.world_mut().spawn(SimulatorRobot {
                team: Team::Opponent,
                player_number,
            });
        }
        app.world_mut()
            .resource_mut::<SimulatorOutgoingMessages>()
            .messages
            .push(SimulatorMessage {
                sender: opponent_robot_id(PlayerNumber::Three),
                message: OutgoingMessage::Hsl(hsl_state_message(PlayerNumber::Three, 1.0, 0.0)),
            });

        app.update();

        let incoming_messages = &app.world().resource::<SimulatorIncomingMessages>().messages;
        assert_eq!(incoming_messages.len(), 1);
        assert_eq!(
            incoming_messages[0].receiver,
            opponent_robot_id(PlayerNumber::Four)
        );
        assert_eq!(
            incoming_messages[0].sender,
            opponent_robot_id(PlayerNumber::Three)
        );
        let game_state = app.world().resource::<SimulatorGameState>();
        assert_eq!(
            game_state
                .game_controller_state
                .hulks_team
                .remaining_amount_of_messages,
            5
        );
        assert_eq!(
            game_state
                .game_controller_state
                .opponent_team
                .remaining_amount_of_messages,
            2
        );
    }

    #[test]
    fn hsl_broadcast_with_empty_budget_is_dropped() {
        let mut app = route_test_app(0);
        app.world_mut()
            .resource_mut::<SimulatorOutgoingMessages>()
            .messages
            .push(SimulatorMessage {
                sender: robot_id(PlayerNumber::Three),
                message: OutgoingMessage::Hsl(hsl_state_message(PlayerNumber::Three, 1.0, 0.0)),
            });

        app.update();

        assert!(
            app.world()
                .resource::<SimulatorIncomingMessages>()
                .messages
                .is_empty()
        );
        assert_eq!(
            app.world()
                .resource::<SimulatorGameState>()
                .game_controller_state
                .hulks_team
                .remaining_amount_of_messages,
            0
        );
    }

    #[test]
    fn game_controller_return_message_does_not_decrement_hsl_budget() {
        let mut app = route_test_app(5);
        app.world_mut()
            .resource_mut::<SimulatorOutgoingMessages>()
            .messages
            .push(SimulatorMessage {
                sender: robot_id(PlayerNumber::Three),
                message: OutgoingMessage::GameController(
                    "127.0.0.1:3838".parse().expect("valid socket address"),
                    hsl_network_messages::GameControllerReturnMessage::default(),
                ),
            });

        app.update();

        assert!(
            app.world()
                .resource::<SimulatorIncomingMessages>()
                .messages
                .is_empty()
        );
        assert_eq!(
            app.world()
                .resource::<SimulatorGameState>()
                .game_controller_state
                .hulks_team
                .remaining_amount_of_messages,
            5
        );
    }

    #[test]
    fn incoming_hsl_messages_update_received_message_cache() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(SimulatorIncomingMessages {
                messages: vec![SimulatorIncomingMessage {
                    receiver: robot_id(PlayerNumber::Four),
                    sender: robot_id(PlayerNumber::Three),
                    message: IncomingMessage::Hsl(hsl_state_message(PlayerNumber::Three, 1.0, 0.0)),
                    received_at: SystemTime::UNIX_EPOCH + Duration::from_secs(2),
                }],
            })
            .insert_resource(SimulatorReceivedHslMessages::default())
            .add_systems(Update, apply_incoming_hsl_messages);

        app.update();

        assert!(
            app.world()
                .resource::<SimulatorIncomingMessages>()
                .messages
                .is_empty()
        );
        let received_hsl_messages = app.world().resource::<SimulatorReceivedHslMessages>();
        assert!(
            received_hsl_messages.messages_by_receiver[&robot_id(PlayerNumber::Four)]
                .contains_key(&robot_id(PlayerNumber::Three))
        );
        assert!(
            received_hsl_messages.player_states_by_receiver[&robot_id(PlayerNumber::Four)]
                [PlayerNumber::Three]
                .is_some()
        );
    }
}
