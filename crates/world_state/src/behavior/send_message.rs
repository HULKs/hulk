use color_eyre::{Result, eyre::Context};
use framework::AdditionalOutput;
use std::{
    net::SocketAddr,
    sync::Arc,
    time::{Duration, SystemTime},
};

use booster::FallDownStateType;
use coordinate_systems::{Field, Ground};
use hardware::NetworkInterface;
use hsl_network_messages::{GameControllerReturnMessage, HulkMessage, PlayerState, StateMessage};
use linear_algebra::Isometry2;
use types::{messages::OutgoingMessage, parameters::HslNetworkParameters, world_state::WorldState};

use crate::behavior::node::Behavior;

impl Behavior {
    pub fn try_sending_game_controller_return_message(
        &mut self,
        world_state: &WorldState,
        game_controller_address: Option<&SocketAddr>,
        hsl_network_parameters: &HslNetworkParameters,
        hardware: &Arc<impl NetworkInterface>,
    ) -> Result<()> {
        let now = world_state.now;

        if !self.is_return_message_cooldown_elapsed(now, hsl_network_parameters) {
            return Ok(());
        }
        let Some(address) = game_controller_address else {
            return Ok(());
        };

        let ground_to_field = ground_to_field_or_initial_pose(world_state);

        let ball_position = world_state
            .ball
            .map(|ball| hsl_network_messages::BallPosition {
                age: now.duration_since(ball.last_seen_ball).unwrap(),
                position: ball.ball_in_ground,
            });

        self.last_system_time_transmitted_game_controller_return_message = Some(now);

        hardware
            .write_to_network(OutgoingMessage::GameController(
                *address,
                GameControllerReturnMessage {
                    player_number: world_state.robot.player_number,
                    fallen: world_state
                        .fall_down_state
                        .is_some_and(|state| state.fall_down_state != FallDownStateType::IsReady),
                    pose: ground_to_field.as_pose(),
                    ball: ball_position,
                },
            ))
            .wrap_err("failed to write GameControllerReturnMessage to hardware")
    }

    fn is_return_message_cooldown_elapsed(
        &self,
        now: SystemTime,
        hsl_network_parameters: &HslNetworkParameters,
    ) -> bool {
        is_cooldown_elapsed(
            now,
            self.last_system_time_transmitted_game_controller_return_message,
            hsl_network_parameters.game_controller_return_message_interval,
        )
    }

    pub fn try_sending_base_message(
        &mut self,
        world_state: &WorldState,
        hsl_network_parameters: &HslNetworkParameters,
        remaining_amount_of_messages: Option<&u16>,
        last_sent_message: &mut AdditionalOutput<HulkMessage>,
        hardware: &Arc<impl NetworkInterface>,
    ) -> Result<()> {
        let now = world_state.now;

        if !self.is_base_message_cooldown_elapsed(now, hsl_network_parameters) {
            return Ok(());
        }
        if remaining_amount_of_messages.is_some_and(|remaining_amount_of_messages| {
            *remaining_amount_of_messages
                < hsl_network_parameters.remaining_amount_of_messages_to_stop_sending
        }) {
            return Ok(());
        }

        let ground_to_field = ground_to_field_or_initial_pose(world_state);
        let pose = ground_to_field.as_pose();

        let ball_position = match world_state.ball {
            Some(ball) => Some(hsl_network_messages::BallPosition {
                age: now.duration_since(ball.last_seen_ball).unwrap(),
                position: ball.ball_in_field,
            }),
            None => None,
        };

        let message = HulkMessage::State(StateMessage {
            player_number: world_state.robot.player_number,
            player_state: PlayerState {
                pose,
                ball_position,
            },
        });

        self.last_transmitted_hsl_message = Some(now);
        last_sent_message.fill_if_subscribed(|| message);

        hardware
            .write_to_network(OutgoingMessage::Hsl(message))
            .wrap_err("failed to write BaseMessage to hardware")
    }

    fn is_base_message_cooldown_elapsed(
        &self,
        now: SystemTime,
        hsl_network_parameters: &HslNetworkParameters,
    ) -> bool {
        is_cooldown_elapsed(
            now,
            self.last_transmitted_hsl_message,
            hsl_network_parameters.hsl_base_message_send_interval,
        )
    }
}

fn ground_to_field_or_initial_pose(world_state: &WorldState) -> Isometry2<Ground, Field> {
    world_state.robot.ground_to_field.unwrap_or_default()
}

fn is_cooldown_elapsed(now: SystemTime, last: Option<SystemTime>, cooldown: Duration) -> bool {
    match last {
        None => true,
        Some(last_time) => now.duration_since(last_time).expect("time ran backwards") > cooldown,
    }
}
