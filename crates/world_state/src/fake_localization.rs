use color_eyre::Result;
use linear_algebra::{Isometry, Isometry2};
use serde::{Deserialize, Serialize};

use context_attribute::context;
use coordinate_systems::{Field, Ground};
use framework::MainOutput;
use hardware::GroundTruthLocalizationInterface;

#[derive(Deserialize, Serialize)]
pub struct Localization {
    last_ground_to_field: Option<Isometry2<Ground, Field>>,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    hardware_interface: HardwareInterface,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub ground_to_field: MainOutput<Option<Isometry2<Ground, Field>>>,
}

impl Localization {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            last_ground_to_field: None,
        })
    }

    pub fn cycle(
        &mut self,
        context: CycleContext<impl GroundTruthLocalizationInterface>,
    ) -> Result<MainOutputs> {
        if let Some(ground_to_field) = context.hardware_interface.read_ground_to_field()? {
            self.last_ground_to_field = Some(ground_to_field);
        }
        let ground_to_field = self
            .last_ground_to_field
            .or(Some(Isometry::identity()));

        Ok(MainOutputs {
            ground_to_field: ground_to_field.into(),
        })
    }
}
