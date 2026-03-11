use std::time::SystemTime;

use booster::Odometer;
use color_eyre::Result;
use linear_algebra::{Isometry2, vector};
use serde::{Deserialize, Serialize};

use context_attribute::context;
use coordinate_systems::{Field, Ground};
use framework::{MainOutput, PerceptionInput};

#[derive(Deserialize, Serialize)]
pub struct Localization {
    last_odometry: Odometer,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    odometer: PerceptionInput<Odometer, "Odometry", "odometer">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub ground_to_field: MainOutput<Option<Isometry2<Ground, Field>>>,
}

impl Localization {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            last_odometry: Default::default(),
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        if let Some(odometer) = context
            .odometer
            .persistent
            .pop_last()
            .and_then(|(_time, values): (SystemTime, Vec<&Odometer>)| values.last().copied())
        {
            self.last_odometry = odometer.clone();
        }

        let ground_to_field = Some(Isometry2::from_parts(
            vector![self.last_odometry.x, self.last_odometry.y],
            self.last_odometry.theta,
        ));

        Ok(MainOutputs {
            ground_to_field: ground_to_field.into(),
        })
    }
}
