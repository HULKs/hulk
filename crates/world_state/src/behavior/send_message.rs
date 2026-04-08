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
use hsl_network_messages::{BaseMessage, GameControllerReturnMessage, HulkMessage};
use linear_algebra::Isometry2;
use types::{
    cycle_time::CycleTime,
    messages::OutgoingMessage,
    parameters::HslNetworkParameters,
    world_state::WorldState,
};

use crate::behavior::node::Behavior;

impl Behavior {
    pub fn try_sending_game_controller_return_message(
        &mut self,
        world_state: &WorldState,
        game_controller_address: Option<&SocketAddr>,
        cycle_time: &CycleTime,
        hsl_network_parameters: &HslNetworkParameters,
        hardware: &Arc<impl NetworkInterface>,
    ) -> Result<()> {
        if !self.is_return_message_cooldown_elapsed(cycle_time, hsl_network_parameters) {
            return Ok(());
        }
        let Some(address) = game_controller_address else {
            return Ok(());
        };

        let ground_to_field = ground_to_field_or_initial_pose(world_state);
        self.last_system_time_transmitted_game_controller_return_message =
            Some(cycle_time.start_time);

        let ball_position = world_state
            .ball
            .map(|ball| hsl_network_messages::BallPosition {
                age: cycle_time
                    .start_time
                    .duration_since(ball.last_seen_ball)
                    .unwrap(),
                position: ball.ball_in_ground,
            });

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
        cycle_time: &CycleTime,
        hsl_network_parameters: &HslNetworkParameters,
    ) -> bool {
        is_cooldown_elapsed(
            cycle_time.start_time,
            self.last_system_time_transmitted_game_controller_return_message,
            hsl_network_parameters.game_controller_return_message_interval,
        )
    }

    pub fn try_sending_base_message(
        &mut self,
        world_state: &WorldState,
        cycle_time: &CycleTime,
        hsl_network_parameters: &HslNetworkParameters,
        remaining_amount_of_messages: Option<&u16>,
        last_sent_message: &mut AdditionalOutput<String>,
        hardware: &Arc<impl NetworkInterface>,
    ) -> Result<()> {
        if !self.is_base_message_cooldown_elapsed(cycle_time, hsl_network_parameters) {
            return Ok(());
        }
        if remaining_amount_of_messages.is_some_and(|remaining_amount_of_messages| {
            *remaining_amount_of_messages
                < hsl_network_parameters.remaining_amount_of_messages_to_stop_sending
        }) {
            return Ok(());
        }

        self.last_transmitted_hsl_message = Some(cycle_time.start_time);

        let ground_to_field = ground_to_field_or_initial_pose(world_state);
        let pose = ground_to_field.as_pose();

        //TODO: Teamball

        let ball_position = world_state
            .ball
            .map(|ball| hsl_network_messages::BallPosition {
                age: cycle_time
                    .start_time
                    .duration_since(ball.last_seen_ball)
                    .unwrap(),
                position: ball.ball_in_field,
            });

        last_sent_message.fill_if_subscribed(|| "Base".to_string());
        hardware
            .write_to_network(OutgoingMessage::Hsl(HulkMessage::Base(BaseMessage {
                player_number: world_state.robot.player_number,
                pose,
                ball_position,
            })))
            .wrap_err("failed to write BaseMessage to hardware")
    }

    fn is_base_message_cooldown_elapsed(
        &self,
        cycle_time: &CycleTime,
        hsl_network_parameters: &HslNetworkParameters,
    ) -> bool {
        is_cooldown_elapsed(
            cycle_time.start_time,
            self.last_transmitted_hsl_message,
            hsl_network_parameters.hsl_base_message_send_interval,
        )
    }
}

// TODO: reintegrate Initial Pose as fallback currently only Default as fallback
fn ground_to_field_or_initial_pose(world_state: &WorldState) -> Isometry2<Ground, Field> {
    world_state.robot.ground_to_field.unwrap_or_default()
}

fn is_cooldown_elapsed(now: SystemTime, last: Option<SystemTime>, cooldown: Duration) -> bool {
    match last {
        None => true,
        Some(last_time) => now.duration_since(last_time).expect("time ran backwards") > cooldown,
    }
}
