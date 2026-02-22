use color_eyre::Result;
use linear_algebra::{Isometry, Isometry2};
use serde::{Deserialize, Serialize};

use context_attribute::context;
use coordinate_systems::{Field, Ground};
use framework::MainOutput;

#[derive(Deserialize, Serialize)]
pub struct Localization {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub ground_to_field: MainOutput<Option<Isometry2<Ground, Field>>>,
}

impl Localization {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, mut _context: CycleContext) -> Result<MainOutputs> {
        let ground_to_field = Some(Isometry::identity());

        Ok(MainOutputs {
            ground_to_field: ground_to_field.into(),
        })
    }
}
