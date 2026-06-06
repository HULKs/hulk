use std::{pin::Pin, sync::Arc};

use color_eyre::Result;
use hsl_network_messages::HulkMessage;
use ros_z::{prelude::*, qos::QosDurability};
use types::{
    ball_position::BallPosition, filtered_game_controller_state::FilteredGameControllerState,
    messages::IncomingMessage, players::Players, time_wrapper::TimeWrapper,
    world_state::PlayerState,
};

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("player_state_receiver").build().await?;
    let filtered_game_controller_state_sub = node
        .subscriber::<TimeWrapper<FilteredGameControllerState>>("filtered_game_controller_state")?
        .build()
        .await?;
    let filtered_message_sub = node
        .subscriber::<TimeWrapper<IncomingMessage>>("filtered_message")?
        .build()
        .await?;
    let player_states_pub = node
        .publisher::<Players<Option<TimeWrapper<PlayerState>>>>("player_states")?
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .build()
        .await?;

    let mut player_states = Players::new(None);
    loop {
        tokio::select! {
            received_game_controller_state = filtered_game_controller_state_sub.recv() => {
                let game_controller_state = received_game_controller_state?;
                clear_penalized_players(&mut player_states, &game_controller_state.inner);
                player_states_pub.publish(&player_states).await?;
            }
            received_message = filtered_message_sub.recv() => {
                apply_message(&mut player_states, received_message?);
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
}
