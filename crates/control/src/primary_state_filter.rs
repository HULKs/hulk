use context_attribute::context;
use framework::{Input, MainOutput, Parameter, RequiredInput};
use spl_network_messages::PlayerNumber;
use types::{Buttons, FilteredGameState, GameControllerState, PrimaryState};

pub struct PrimaryStateFilter {}

#[context]
pub struct NewContext {
    pub player_number: Parameter<PlayerNumber, "player_number">,
}

#[context]
pub struct CycleContext {
    pub buttons: Input<Buttons, "buttons">,
    pub filtered_game_state: RequiredInput<Option<FilteredGameState>, "filtered_game_state?">,
    pub game_controller_state: RequiredInput<Option<GameControllerState>, "game_controller_state?">,
    pub has_ground_contact: Input<bool, "has_ground_contact">,

    pub player_number: Parameter<PlayerNumber, "player_number">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub primary_state: MainOutput<Option<PrimaryState>>,
}

impl PrimaryStateFilter {
    pub fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, _context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
