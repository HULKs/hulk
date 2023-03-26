use std::time::SystemTime;

use color_eyre::Result;
use context_attribute::context;
use framework::{MainOutput, PerceptionInput};
use types::{messages::IncomingMessage, CycleTime, GameControllerState, SensorData};

pub struct GameControllerFilter {
    game_controller_state: Option<GameControllerState>,
    last_game_state_change: Option<SystemTime>,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    pub sensor_data: Input<SensorData, "sensor_data">,
    pub cycle_time: Input<CycleTime, "cycle_time">,
    pub network_message: PerceptionInput<IncomingMessage, "SplNetwork", "message">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub game_controller_state: MainOutput<Option<GameControllerState>>,
}

impl GameControllerFilter {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            game_controller_state: None,
            last_game_state_change: None,
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        for game_controller_state_message in context
            .network_message
            .persistent
            .values()
            .flatten()
            .filter_map(|message| match message {
                IncomingMessage::GameController(message) => Some(message),
                IncomingMessage::Spl(_) => None,
            })
        {
            let game_state_changed = match &self.game_controller_state {
                Some(game_controller_state) => {
                    game_controller_state.game_state != game_controller_state_message.game_state
                }
                None => true,
            };
            if game_state_changed {
                self.last_game_state_change = Some(context.cycle_time.start_time);
            }
            self.game_controller_state = Some(GameControllerState {
                game_state: game_controller_state_message.game_state,
                game_phase: game_controller_state_message.game_phase,
                kicking_team: game_controller_state_message.kicking_team,
                last_game_state_change: self.last_game_state_change.unwrap(),
                penalties: game_controller_state_message.hulks_team.clone().into(),
                remaining_amount_of_messages: game_controller_state_message
                    .hulks_team
                    .remaining_amount_of_messages,
                sub_state: game_controller_state_message.sub_state,
            });
        }
        Ok(MainOutputs {
            game_controller_state: self.game_controller_state.into(),
        })
    }
}
