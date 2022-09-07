use context_attribute::context;
use framework::{MainOutput, PerceptionInput, RequiredInput};
use types::{Buttons, GameControllerState, SensorData};

// TODO: Check this
pub struct GameControllerFilter {
    game_controller_state: Option<GameControllerState>,
    last_game_state_change: Option<SystemTime>,
}

#[context]
pub struct NewContext {}

#[context]
pub struct CycleContext {
    pub sensor_data: RequiredInput<SensorData, "sensor_data">,

    pub game_controller_state_message: PerceptionInput<
        spl_network::GameControllerStateMessage, // TODO
        "SPLNetwork",                            // ?
        "game_controller_state_message",
    >,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub game_controller_state: MainOutput<Option<GameControllerState>>,
}

impl GameControllerFilter {
    pub fn new(context: NewContext) -> anyhow::Result<Self> {
        Ok(Self)
    }

    pub fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
