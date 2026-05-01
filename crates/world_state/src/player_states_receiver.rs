use std::net::SocketAddr;

use booster::FallDownState;
use color_eyre::Result;
use context_attribute::context;
use coordinate_systems::{Field, Ground};
use framework::{MainOutput, PerceptionInput};
use hardware::NetworkInterface;
use hsl_network_messages::{HulkMessage, PlayerNumber};
use linear_algebra::Isometry2;
use linear_algebra::Vector2;
use serde::{Deserialize, Serialize};
use types::{
    ball_position::BallPosition, cycle_time::CycleTime, messages::IncomingMessage,
    parameters::HslNetworkParameters, players::Players, world_state::PlayerState,
};

#[derive(Serialize, Deserialize)]
pub struct PlayerStatesReceiver {
    pub last_player_states: Players<Option<PlayerState>>,
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
    pub player_states: MainOutput<Players<PlayerState>>,
}

impl PlayerStatesReceiver {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            last_player_states: Players {
                one: None,
                two: None,
                three: None,
                four: None,
                five: None,
            },
        })
    }

    pub fn cycle(&mut self, context: CycleContext<impl NetworkInterface>) -> Result<MainOutputs> {
        let messages = context
            .network_message
            .persistent
            .values()
            .flat_map(|messages| messages.iter().filter_map(|message| *message))
            .filter_map(|message| match message {
                IncomingMessage::Hsl(message) => Some(*message),
                _ => None,
            });

        let mut player_states = self.last_player_states;
        for message in messages {
            match message {
                HulkMessage::State(base_message) => {
                    player_states[base_message.player_number] = Some(PlayerState {
                        pose: base_message.pose,
                        ball_position: base_message.ball_position.map(
                            |ball| BallPosition::<Field> {
                                position: ball.position,
                                velocity: Vector2::zeros(),
                                last_seen: context.cycle_time.start_time - ball.age,
                            },
                        ),
                    });
                }
            }
        }
        self.last_player_states = player_states;

        Ok(MainOutputs {
            player_states: player_states.map(Option::unwrap_or_default).into(),
        })
    }
}
