use color_eyre::{Result, eyre::Context};
use framework::AdditionalOutput;
use std::{
    net::SocketAddr,
    sync::Arc,
    time::{Duration, SystemTime},
};

use booster::FallDownStateType;
use hardware::NetworkInterface;
use hsl_network_messages::{GameControllerReturnMessage, HulkMessage, StateMessage};
use types::{messages::OutgoingMessage, parameters::HslNetworkParameters, world_state::WorldState};

use crate::behavior::node::Behavior;

pub struct CommunicationInput<'a> {
    pub world_state: &'a WorldState,
    pub game_controller_address: Option<SocketAddr>,
    pub hsl_network_parameters: &'a HslNetworkParameters,
    pub remaining_amount_of_messages: Option<u16>,
}

pub struct CommunicationOutput {
    pub outgoing_messages: Vec<OutgoingMessage>,
    pub last_sent_message: Option<HulkMessage>,
}

impl Behavior {
    pub fn plan_communication(&mut self, input: CommunicationInput<'_>) -> CommunicationOutput {
        let mut outgoing_messages = Vec::new();
        let mut last_sent_message = None;

        if let Some(message) = self.plan_game_controller_return_message(
            input.world_state,
            input.game_controller_address,
            input.hsl_network_parameters,
        ) {
            outgoing_messages.push(message);
        }

        if let Some(message) = self.plan_state_message(
            input.world_state,
            input.hsl_network_parameters,
            input.remaining_amount_of_messages,
        ) {
            last_sent_message = Some(message.clone());
            outgoing_messages.push(OutgoingMessage::Hsl(message));
        }

        CommunicationOutput {
            outgoing_messages,
            last_sent_message,
        }
    }

    fn plan_game_controller_return_message(
        &mut self,
        world_state: &WorldState,
        game_controller_address: Option<SocketAddr>,
        hsl_network_parameters: &HslNetworkParameters,
    ) -> Option<OutgoingMessage> {
        let now = world_state.now.to_wallclock();

        if !self.is_return_message_cooldown_elapsed(now, hsl_network_parameters) {
            return None;
        }
        let address = game_controller_address?;

        let ground_to_field = world_state.robot.ground_to_field.unwrap_or_default();

        let ball_position = world_state
            .ball
            .map(|ball| hsl_network_messages::BallPosition {
                age: now.duration_since(ball.last_seen_ball).unwrap(),
                position: ball.ball_in_ground,
            });

        self.last_sent_game_controller_return_message_time = Some(now);

        Some(OutgoingMessage::GameController(
            address,
            GameControllerReturnMessage {
                player_number: world_state.robot.player_number,
                fallen: world_state
                    .fall_down_state
                    .is_some_and(|state| state.fall_down_state != FallDownStateType::IsReady),
                pose: ground_to_field.as_pose(),
                ball: ball_position,
            },
        ))
    }

    fn plan_state_message(
        &mut self,
        world_state: &WorldState,
        hsl_network_parameters: &HslNetworkParameters,
        remaining_amount_of_messages: Option<u16>,
    ) -> Option<HulkMessage> {
        let now = world_state.now.to_wallclock();

        if !self.is_state_message_cooldown_elapsed(now, hsl_network_parameters) {
            return None;
        }
        if remaining_amount_of_messages.is_none_or(|remaining_amount_of_messages| {
            remaining_amount_of_messages
                < hsl_network_parameters.remaining_amount_of_messages_to_stop_sending
        }) {
            return None;
        }

        let ground_to_field = world_state.robot.ground_to_field.unwrap_or_default();

        let pose = ground_to_field.as_pose();

        let ball_position = world_state
            .ball
            .map(|ball| hsl_network_messages::BallPosition {
                age: now.duration_since(ball.last_seen_ball).unwrap(),
                position: ball.ball_in_field,
            });

        let message = HulkMessage::State(StateMessage {
            player_number: world_state.robot.player_number,
            pose,
            ball_position,
        });

        self.last_sent_hsl_message_time = Some(now);
        Some(message)
    }

    pub fn send_game_controller_return_message(
        &mut self,
        world_state: &WorldState,
        game_controller_address: Option<&SocketAddr>,
        hsl_network_parameters: &HslNetworkParameters,
        hardware: &Arc<impl NetworkInterface>,
    ) -> Result<()> {
        if let Some(message) = self.plan_game_controller_return_message(
            world_state,
            game_controller_address.copied(),
            hsl_network_parameters,
        ) {
            hardware
                .write_to_network(message)
                .wrap_err("failed to write GameControllerReturnMessage to hardware")?;
        }
        Ok(())
    }

    fn is_return_message_cooldown_elapsed(
        &self,
        now: SystemTime,
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
        world_state: &WorldState,
        hsl_network_parameters: &HslNetworkParameters,
        remaining_amount_of_messages: Option<&u16>,
        last_sent_message: &mut AdditionalOutput<HulkMessage>,
        hardware: &Arc<impl NetworkInterface>,
    ) -> Result<()> {
        if let Some(message) = self.plan_state_message(
            world_state,
            hsl_network_parameters,
            remaining_amount_of_messages.copied(),
        ) {
            last_sent_message.fill_if_subscribed(|| message.clone());
            hardware
                .write_to_network(OutgoingMessage::Hsl(message))
                .wrap_err("failed to write StateMessage to hardware")?;
        }
        Ok(())
    }

    fn is_state_message_cooldown_elapsed(
        &self,
        now: SystemTime,
        hsl_network_parameters: &HslNetworkParameters,
    ) -> bool {
        is_cooldown_elapsed(
            now,
            self.last_sent_hsl_message_time,
            hsl_network_parameters.hsl_state_message_send_interval,
        )
    }
}

fn is_cooldown_elapsed(now: SystemTime, last: Option<SystemTime>, cooldown: Duration) -> bool {
    match last {
        None => true,
        Some(last_time) => now.duration_since(last_time).expect("time ran backwards") > cooldown,
    }
}
