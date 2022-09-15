use framework::{
    MainOutput, Parameter, OptionalInput
};

pub struct PrimaryStateFilter {}

#[context]
pub struct NewContext {
    pub player_number: Parameter<PlayerNumber, "player_number">,
}

#[context]
pub struct CycleContext {


    pub buttons: OptionalInput<Buttons, "buttons">,
    pub filtered_game_state: OptionalInput<FilteredGameState, "filtered_game_state">,
    pub game_controller_state: OptionalInput<GameControllerState, "game_controller_state">,
    pub has_ground_contact: OptionalInput<bool, "has_ground_contact">,

    pub player_number: Parameter<PlayerNumber, "player_number">,



}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub primary_state: MainOutput<PrimaryState>,
}

impl PrimaryStateFilter {
    pub fn new(context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
