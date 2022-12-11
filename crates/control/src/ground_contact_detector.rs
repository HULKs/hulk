use std::time::Duration;

use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use types::{SensorData, SolePressure};

pub struct GroundContactDetector {}

#[context]
pub struct CreationContext {
    pub hysteresis: Parameter<f32, "control/ground_contact_detector/hysteresis">,
    pub pressure_threshold: Parameter<f32, "control/ground_contact_detector/pressure_threshold">,
    pub timeout: Parameter<Duration, "control/ground_contact_detector/timeout">,
}

#[context]
pub struct CycleContext {
    pub sensor_data: Input<SensorData, "sensor_data">,
    pub sole_pressure: Input<SolePressure, "sole_pressure">,

    pub hysteresis: Parameter<f32, "control/ground_contact_detector/hysteresis">,
    pub pressure_threshold: Parameter<f32, "control/ground_contact_detector/pressure_threshold">,
    pub timeout: Parameter<Duration, "control/ground_contact_detector/timeout">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub has_ground_contact: MainOutput<bool>,
}

impl GroundContactDetector {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, _context: CycleContext) -> Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
