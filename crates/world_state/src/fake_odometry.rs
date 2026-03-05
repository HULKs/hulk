use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use nalgebra::Isometry2;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct Odometry {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub current_odometry_to_last_odometry: MainOutput<Option<Isometry2<f32>>>,
}

impl Odometry {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, mut _context: CycleContext) -> Result<MainOutputs> {
        Ok(MainOutputs {
            current_odometry_to_last_odometry: None.into(),
        })
    }
}
