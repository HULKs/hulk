use context_attribute::context;
use framework::{MainOutput, OptionalInput, Parameter, PersistentState};
use nalgebra::Isometry2;
use spl_network_messages::PlayerNumber;
use types::{
    configuration::GameStateFilter as GameStateFilterConfiguration, BallPosition, Buttons,
    FieldDimensions, FilteredGameState, FilteredWhistle, GameControllerState, SensorData,
};

pub struct GameStateFilter {}

#[context]
pub struct NewContext {
    pub config: Parameter<GameStateFilterConfiguration, "control/game_state_filter">,
    pub field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    pub player_number: Parameter<PlayerNumber, "player_number">,

    pub robot_to_field: PersistentState<Isometry2<f32>, "robot_to_field">,
}

#[context]
pub struct CycleContext {
    pub ball_position: OptionalInput<BallPosition, "ball_position?">,
    pub buttons: OptionalInput<Buttons, "buttons?">,
    pub filtered_whistle: OptionalInput<FilteredWhistle, "filtered_whistle?">,
    pub game_controller_state: OptionalInput<Option<GameControllerState>, "game_controller_state?">,
    pub sensor_data: OptionalInput<SensorData, "sensor_data?">,

    pub config: Parameter<GameStateFilterConfiguration, "control/game_state_filter">,
    pub field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    pub player_number: Parameter<PlayerNumber, "player_number">,

    pub robot_to_field: PersistentState<Isometry2<f32>, "robot_to_field">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub filtered_game_state: MainOutput<FilteredGameState>,
}

impl GameStateFilter {
    pub fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, _context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
