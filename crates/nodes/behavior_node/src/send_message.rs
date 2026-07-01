use std::{net::SocketAddr, time::Duration};

use booster::FallDownStateType;
use hsl_network_messages::{GameControllerReturnMessage, HulkMessage, StateMessage};
use ros_z::time::Time;
use types::{messages::OutgoingMessage, parameters::HslNetworkParameters};

use crate::node::Blackboard;

impl Blackboard {
    pub fn game_controller_return_message(
        &mut self,
        game_controller_address: Option<&SocketAddr>,
    ) -> Option<OutgoingMessage> {
        let now = self.world_state.now;

        if !self.is_return_message_cooldown_elapsed(now, &self.hsl_network_parameters) {
            return None;
        }
        let address = game_controller_address?;

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

        Some(OutgoingMessage::GameController(
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

    pub fn state_message(&mut self) -> Option<OutgoingMessage> {
        let now = self.world_state.now;
        let remaining_amount_of_messages = self
            .world_state
            .filtered_game_controller_state
            .as_ref()
            .map(|state| state.remaining_number_of_messages);

        if !self.is_state_message_cooldown_elapsed(now, &self.hsl_network_parameters) {
            return None;
        }
        if remaining_amount_of_messages.is_none_or(|remaining_amount_of_messages| {
            remaining_amount_of_messages
                < self
                    .hsl_network_parameters
                    .remaining_amount_of_messages_to_stop_sending
        }) {
            return None;
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

        Some(OutgoingMessage::Hsl(message))
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
