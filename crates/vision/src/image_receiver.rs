use std::sync::Arc;

use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;

pub struct ImageReceiver {}

#[context]
pub struct NewContext {}

#[context]
pub struct CycleContext {}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub image: MainOutput<Arc<bool>>,
}

impl ImageReceiver {
    pub fn new(_context: NewContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, _context: CycleContext) -> Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
