use color_eyre::Result;
use context_attribute::context;
use framework::{MainOutput, PerceptionInput};
use types::{Ball, Leds, PrimaryState, SensorData};

pub struct LedStatus {}

#[context]
pub struct NewContext {}

#[context]
pub struct CycleContext {
    pub primary_state: RequiredInput<Option<PrimaryState>, "primary_state?">,
    pub sensor_data: Input<SensorData, "sensor_data">,

    pub balls_bottom: PerceptionInput<Option<Vec<Ball>>, "VisionBottom", "balls?">,
    pub balls_top: PerceptionInput<Option<Vec<Ball>>, "VisionTop", "balls?">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub leds: MainOutput<Leds>,
}

impl LedStatus {
    pub fn new(_context: NewContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, _context: CycleContext) -> Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
