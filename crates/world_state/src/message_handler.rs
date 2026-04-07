use std::{
    net::SocketAddr,
    time::{Duration, SystemTime},
};

use booster::FallDownState;
use color_eyre::{Result, eyre::Context};
use context_attribute::context;
use coordinate_systems::{Field, Ground};
use framework::{MainOutput, PerceptionInput};
use hardware::NetworkInterface;
use hsl_network_messages::{GameControllerReturnMessage, HulkMessage, PlayerNumber};
use linear_algebra::{Isometry2, Point2};
use types::{
    ball_position::BallPosition,
    cycle_time::CycleTime,
    messages::{IncomingMessage, OutgoingMessage},
    parameters::HslNetworkParameters,
};

pub struct PlayerMessageOutput {
    player_number: PlayerNumber,
    position: Point2<Field>,
}

pub struct MessageHandler {
    last_system_time_transmitted_game_controller_return_message: Option<SystemTime>,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    game_controller_address: Input<Option<SocketAddr>, "game_controller_address?">,
    cycle_time: Input<CycleTime, "cycle_time">,
    ground_to_field: Input<Option<Isometry2<Ground, Field>>, "ground_to_field?">,
    ball_position: Input<Option<BallPosition<Ground>>, "ball_position?">,

    fall_down_state: PerceptionInput<Option<FallDownState>, "FallDownState", "fall_down_state?">,
    network_message: PerceptionInput<Option<IncomingMessage>, "HslNetwork", "filtered_message?">,

    player_number: Parameter<PlayerNumber, "player_number">,
    hsl_network_parameters: Parameter<HslNetworkParameters, "hsl_network">,

    hardware: HardwareInterface,
}

#[context]
pub struct MainOutputs {
    pub player_messages: MainOutput<Vec<PlayerMessageOutput>>,
}

impl MessageHandler {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            last_system_time_transmitted_game_controller_return_message: None,
        })
    }

    pub fn cycle(&mut self, context: CycleContext<impl NetworkInterface>) -> Result<MainOutputs> {
        self.try_sending_game_controller_return_message(&context)?;

        let messages = context
            .network_message
            .persistent
            .values()
            .flat_map(|messages| messages.iter().filter_map(|message| *message))
            .filter_map(|message| match message {
                IncomingMessage::Hsl(message) => Some(*message),
                _ => None,
            });

        let player_messages = messages
            .filter_map(|message| match message {
                HulkMessage::Base(base_message)
                    if base_message.player_number != *context.player_number =>
                {
                    Some(PlayerMessageOutput {
                        player_number: base_message.player_number,
                        position: context
                            .ground_to_field
                            .map_or(Point2::origin(), |ground_to_field| {
                                ground_to_field * base_message.pose.position()
                            }),
                    })
                }
                _ => None,
            })
            .collect();

        Ok(MainOutputs {
            player_messages: player_messages.into(),
        })
    }

    fn try_sending_game_controller_return_message(
        &mut self,
        context: &CycleContext<impl NetworkInterface>,
    ) -> Result<()> {
        if !self.is_return_message_cooldown_elapsed(context) {
            return Ok(());
        }
        let Some(address) = context.game_controller_address else {
            return Ok(());
        };
        let ground_to_field = ground_to_field_or_initial_pose(context);

        self.last_system_time_transmitted_game_controller_return_message =
            Some(context.cycle_time.start_time);
        context
            .hardware
            .write_to_network(OutgoingMessage::GameController(
                *address,
                GameControllerReturnMessage {
                    player_number: *context.player_number,
                    fallen: context
                        .fall_down_state
                        .persistent
                        .iter()
                        .flat_map(|(_, messages)| messages.iter())
                        .any(|message| {
                            matches!(
                                message,
                                Some(FallDownState {
                                    fall_down_state: booster::FallDownStateType::IsFalling
                                        | booster::FallDownStateType::HasFallen
                                        | booster::FallDownStateType::IsGettingUp,
                                    ..
                                })
                            )
                        }),
                    pose: ground_to_field.as_pose(),
                    ball: seen_ball_to_game_controller_ball_position(
                        context.ball_position,
                        context.cycle_time.start_time,
                    ),
                },
            ))
            .wrap_err("failed to write GameControllerReturnMessage to hardware")
    }
    fn is_return_message_cooldown_elapsed(
        &self,
        context: &CycleContext<impl NetworkInterface>,
    ) -> bool {
        is_cooldown_elapsed(
            context.cycle_time.start_time,
            self.last_system_time_transmitted_game_controller_return_message,
            context
                .hsl_network_parameters
                .game_controller_return_message_interval,
        )
    }
}

// TODO: reintegrate Initial Pose as fallback currently only Default as fallback
fn ground_to_field_or_initial_pose(
    context: &CycleContext<'_, impl NetworkInterface>,
) -> Isometry2<Ground, Field> {
    context.ground_to_field.copied().unwrap_or_default()
}

fn seen_ball_to_game_controller_ball_position(
    ball: Option<&BallPosition<Ground>>,
    cycle_start_time: SystemTime,
) -> Option<hsl_network_messages::BallPosition<Ground>> {
    ball.map(|ball| hsl_network_messages::BallPosition {
        age: cycle_start_time.duration_since(ball.last_seen).unwrap(),
        position: ball.position,
    })
}

fn is_cooldown_elapsed(now: SystemTime, last: Option<SystemTime>, cooldown: Duration) -> bool {
    match last {
        None => true,
        Some(last_time) => now.duration_since(last_time).expect("time ran backwards") > cooldown,
    }
}
