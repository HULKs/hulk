use std::time::{Duration, SystemTime, UNIX_EPOCH};

use color_eyre::Result;
use context_attribute::context;
use filtering::hysteresis::greater_than_with_hysteresis;
use framework::MainOutput;
use types::{cycle_time::CycleTime, sole_pressure::SolePressure};

pub struct GroundContactDetector {
    last_has_pressure: bool,
    last_time_switched: SystemTime,
    has_ground_contact: bool,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    sole_pressure: Input<SolePressure, "sole_pressure">,
    cycle_time: Input<CycleTime, "cycle_time">,

    hysteresis: Parameter<f32, "ground_contact_detector.hysteresis">,
    pressure_threshold: Parameter<f32, "ground_contact_detector.pressure_threshold">,
    timeout: Parameter<Duration, "ground_contact_detector.timeout">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub has_ground_contact: MainOutput<bool>,
}

impl GroundContactDetector {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            last_has_pressure: false,
            last_time_switched: UNIX_EPOCH,
            has_ground_contact: false,
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let has_pressure = greater_than_with_hysteresis(
            self.last_has_pressure,
            context.sole_pressure.total(),
            *context.pressure_threshold,
            *context.hysteresis,
        );
        if self.last_has_pressure != has_pressure {
            self.last_time_switched = context.cycle_time.start_time;
        }
        if context
            .cycle_time
            .start_time
            .duration_since(self.last_time_switched)
            .expect("time ran backwards")
            > *context.timeout
        {
            self.has_ground_contact = has_pressure;
        }
        self.last_has_pressure = has_pressure;

        Ok(MainOutputs {
            has_ground_contact: self.has_ground_contact.into(),
        })
    }
}
