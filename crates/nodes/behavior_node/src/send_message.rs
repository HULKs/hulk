use color_eyre::{Result, eyre::Context};
use framework::AdditionalOutput;
use std::{net::SocketAddr, sync::Arc, time::Duration};

use booster::FallDownStateType;
use hardware::NetworkInterface;
use hsl_network_messages::{GameControllerReturnMessage, HulkMessage, StateMessage};
use ros_z::time::Time;
use types::{messages::OutgoingMessage, parameters::HslNetworkParameters};

use crate::node::Blackboard;

impl Blackboard {
    pub fn send_game_controller_return_message(
        &mut self,
        game_controller_address: Option<&SocketAddr>,
        hardware: &Arc<impl NetworkInterface>,
    ) -> Result<()> {
        let now = self.world_state.now;

        if !self.is_return_message_cooldown_elapsed(now, &self.parameters.hsl_network) {
            return Ok(());
        }
        let Some(address) = game_controller_address else {
            return Ok(());
        };

        let ground_to_field = self.world_state.robot.ground_to_field.unwrap_or_default();

        let ball_position = self
            .world_state
            .ball
            .map(|ball| hsl_network_messages::BallPosition {
                age: now
                    .to_wallclock()
                    .duration_since(ball.last_seen_ball)
                    .unwrap(),
                position: ball.ball_in_ground,
            });

        self.last_sent_game_controller_return_message_time = Some(now);

        hardware
            .write_to_network(OutgoingMessage::GameController(
                *address,
                GameControllerReturnMessage {
                    player_number: self.world_state.robot.player_number,
                    fallen: self
                        .world_state
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
        now: Time,
        hsl_network_parameters: &HslNetworkParameters,
    ) -> bool {
        is_cooldown_elapsed(
            now,
            self.last_sent_game_controller_return_message_time,
            hsl_network_parameters.game_controller_return_message_interval,
        )
    }

    pub fn send_state_message(
        &mut self,
        remaining_amount_of_messages: Option<&u16>,
        last_sent_message: &mut AdditionalOutput<HulkMessage>,
        hardware: &Arc<impl NetworkInterface>,
    ) -> Result<()> {
        let now = self.world_state.now;

        if !self.is_state_message_cooldown_elapsed(now, &self.parameters.hsl_network) {
            return Ok(());
        }
        if remaining_amount_of_messages.is_none_or(|remaining_amount_of_messages| {
            *remaining_amount_of_messages
                < self
                    .parameters
                    .hsl_network
                    .remaining_amount_of_messages_to_stop_sending
        }) {
            return Ok(());
        }

        let ground_to_field = self.world_state.robot.ground_to_field.unwrap_or_default();

        let pose = ground_to_field.as_pose();

        let ball_position = self
            .world_state
            .ball
            .map(|ball| hsl_network_messages::BallPosition {
                age: now
                    .to_wallclock()
                    .duration_since(ball.last_seen_ball)
                    .unwrap(),
                position: ball.ball_in_field,
            });

        let message = HulkMessage::State(StateMessage {
            player_number: self.world_state.robot.player_number,
            pose,
            ball_position,
        });

        self.last_sent_hsl_message_time = Some(now);
        last_sent_message.fill_if_subscribed(|| message);

        hardware
            .write_to_network(OutgoingMessage::Hsl(message))
            .wrap_err("failed to write StateMessage to hardware")
    }

    fn is_state_message_cooldown_elapsed(
        &self,
        now: Time,
        hsl_network_parameters: &HslNetworkParameters,
    ) -> bool {
        is_cooldown_elapsed(
            now,
            self.last_sent_hsl_message_time,
            hsl_network_parameters.hsl_state_message_send_interval,
        )
    }
}

fn is_cooldown_elapsed(now: Time, last: Option<Time>, cooldown: Duration) -> bool {
    match last {
        None => true,
        Some(last_time) => now.duration_since(last_time) > cooldown,
    }
}
