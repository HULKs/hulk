use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use types::{GameControllerState, SensorData};

pub struct GameControllerFilter {}

#[context]
pub struct NewContext {}

#[context]
pub struct CycleContext {
    pub sensor_data: Input<SensorData, "sensor_data">,
    // TODO: wieder einkommentieren
    // pub game_controller_state_message:
    //     PerceptionInput<GameControllerStateMessage, "SplNetwork", "game_controller_state_message">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub game_controller_state: MainOutput<Option<GameControllerState>>,
}

impl GameControllerFilter {
    pub fn new(_context: NewContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, _context: CycleContext) -> Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
