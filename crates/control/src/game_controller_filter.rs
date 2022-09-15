use context_attribute::context;
use framework::{MainOutput, OptionalInput, PerceptionInput};

pub struct GameControllerFilter {}

#[context]
pub struct NewContext {}

#[context]
pub struct CycleContext {
    pub sensor_data: OptionalInput<SensorData, "sensor_data">,

    pub game_controller_state_message:
        PerceptionInput<GameControllerStateMessage, "SplNetwork", "game_controller_state_message">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub game_controller_state: MainOutput<GameControllerState>,
}

impl GameControllerFilter {
    pub fn new(context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
