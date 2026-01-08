use color_eyre::Result;
use serde::{Deserialize, Serialize};

use context_attribute::context;
use framework::MainOutput;

use types::ball_detection::BallPercept;

#[derive(Deserialize, Serialize)]
pub struct BallProjector {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub balls: MainOutput<Option<Vec<BallPercept>>>,
}

impl BallProjector {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle() -> Result<MainOutputs> {
        Ok(MainOutputs { balls: None.into() })
    }
}
