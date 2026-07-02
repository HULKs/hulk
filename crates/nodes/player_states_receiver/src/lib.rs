use std::{pin::Pin, sync::Arc, time::Duration};

use color_eyre::Result;
use hsl_network_messages::{HulkMessage, PlayerNumber};
use ros_z::{prelude::*, qos::QosDurability, time::Time};
use serde::{Deserialize, Serialize};
use types::{
    ball_position::BallPosition, filtered_game_controller_state::FilteredGameControllerState,
    messages::IncomingMessage, players::Players, time_wrapper::TimeWrapper,
    world_state::PlayerState,
};

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub maximum_age: Duration,
}

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("player_states_receiver").build().await?;

    let parameters = node.bind_parameter_as::<Parameters>("player_states_receiver")?;
    let filtered_game_controller_state_sub = node
        .subscriber::<FilteredGameControllerState>("filtered_game_controller_state")
        .build()
        .await?;
    let filtered_message_sub = node
        .subscriber::<TimeWrapper<IncomingMessage>>("filtered_message")
        .build()
        .await?;
    let player_states_pub = node
        .publisher::<Players<Option<TimeWrapper<PlayerState>>>>("player_states")
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .build()
        .await?;

    let mut player_states = Players::new(None);
    let mut expiry_tick = node.create_timer(Duration::from_millis(100));
    loop {
        tokio::select! {
            _ = expiry_tick.tick() => {
                let maximum_age = parameters.snapshot().typed().maximum_age;
                if clear_old_player_states(&mut player_states, node.clock().now(), maximum_age) {
                    player_states_pub.publish(&player_states).await?;
                }
            }
            received_game_controller_state = filtered_game_controller_state_sub.recv() => {
                let game_controller_state = received_game_controller_state?;
                clear_penalized_players(&mut player_states, &game_controller_state);
                let maximum_age = parameters.snapshot().typed().maximum_age;
                clear_old_player_states(&mut player_states, node.clock().now(), maximum_age);
                player_states_pub.publish(&player_states).await?;
            }
            received_message = filtered_message_sub.recv() => {
                apply_message(&mut player_states, received_message?);
                let maximum_age = parameters.snapshot().typed().maximum_age;
                clear_old_player_states(&mut player_states, node.clock().now(), maximum_age);
                player_states_pub.publish(&player_states).await?;
            }
        }
    }
}

fn apply_message(
    player_states: &mut Players<Option<TimeWrapper<PlayerState>>>,
    message: TimeWrapper<IncomingMessage>,
) {
    let TimeWrapper {
        time,
        inner: IncomingMessage::Hsl(HulkMessage::State(state_message)),
    } = message
    else {
        return;
    };

    player_states[state_message.player_number] = Some(TimeWrapper {
        time,
        inner: PlayerState {
            pose: state_message.pose,
            ball_position: state_message
                .ball_position
                .map(|ball| BallPosition::from_network_ball(ball, time)),
        },
    });
}

fn clear_penalized_players(
    player_states: &mut Players<Option<TimeWrapper<PlayerState>>>,
    game_controller_state: &FilteredGameControllerState,
) {
    for (player_number, penalty) in game_controller_state.penalties.iter() {
        if penalty.is_some() {
            player_states[player_number] = None;
        }
    }
}

fn clear_old_player_states(
    player_states: &mut Players<Option<TimeWrapper<PlayerState>>>,
    now: Time,
    maximum_age: Duration,
) -> bool {
    let mut removed_any = false;
    for player_number in [
        PlayerNumber::One,
        PlayerNumber::Two,
        PlayerNumber::Three,
        PlayerNumber::Four,
        PlayerNumber::Five,
    ] {
        let player_state = &mut player_states[player_number];
        if player_state
            .as_ref()
            .is_some_and(|player_state| now.duration_since(player_state.time) >= maximum_age)
        {
            *player_state = None;
            removed_any = true;
        }
    }
    removed_any
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    use hsl_network_messages::{Penalty, PlayerNumber, StateMessage};
    use linear_algebra::Pose2;
    use ros_z::time::Time;
    use types::messages::IncomingMessage;

    #[test]
    fn state_message_updates_player_with_receive_time() {
        let mut states = Players::new(None);
        let time = Time::from_nanos(42);
        let pose = Pose2::default();

        apply_message(
            &mut states,
            TimeWrapper {
                time,
                inner: IncomingMessage::Hsl(HulkMessage::State(StateMessage {
                    player_number: PlayerNumber::Two,
                    pose,
                    head_yaw: 0.0,
                    ball_position: None,
                })),
            },
        );

        let player = states[PlayerNumber::Two].as_ref().unwrap();
        assert_eq!(player.time, time);
        assert_eq!(player.inner.pose, pose);
    }

    #[test]
    fn penalized_players_are_cleared() {
        let mut states = Players::new(None);
        states[PlayerNumber::Two] = Some(TimeWrapper {
            time: Time::from_nanos(10),
            inner: PlayerState::default(),
        });
        states[PlayerNumber::Three] = Some(TimeWrapper {
            time: Time::from_nanos(20),
            inner: PlayerState::default(),
        });

        let game_controller_state = FilteredGameControllerState {
            penalties: Players {
                two: Some(Penalty::PickUp {
                    remaining: Duration::ZERO,
                }),
                ..Default::default()
            },
            ..Default::default()
        };

        clear_penalized_players(&mut states, &game_controller_state);

        assert!(states[PlayerNumber::Two].is_none());
        assert!(states[PlayerNumber::Three].is_some());
    }

    #[test]
    fn old_player_states_are_cleared() {
        let mut states = Players::new(None);
        states[PlayerNumber::Two] = Some(TimeWrapper {
            time: Time::from_nanos(1_000_000_000),
            inner: PlayerState::default(),
        });
        states[PlayerNumber::Three] = Some(TimeWrapper {
            time: Time::from_nanos(1_500_000_000),
            inner: PlayerState::default(),
        });

        let removed_any = clear_old_player_states(
            &mut states,
            Time::from_nanos(2_000_000_000),
            Duration::from_secs(1),
        );

        assert!(removed_any);
        assert!(states[PlayerNumber::Two].is_none());
        assert!(states[PlayerNumber::Three].is_some());
    }

    #[test]
    fn fresh_player_states_are_kept() {
        let mut states = Players::new(None);
        states[PlayerNumber::Two] = Some(TimeWrapper {
            time: Time::from_nanos(1_000_000_001),
            inner: PlayerState::default(),
        });

        let removed_any = clear_old_player_states(
            &mut states,
            Time::from_nanos(2_000_000_000),
            Duration::from_secs(1),
        );

        assert!(!removed_any);
        assert!(states[PlayerNumber::Two].is_some());
    }
}
