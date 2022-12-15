use std::time::{Duration, SystemTime, UNIX_EPOCH};

use color_eyre::Result;
use context_attribute::context;
use filtering::greater_than_with_hysteresis;
use framework::MainOutput;
use types::{CycleInfo, SensorData, SolePressure};

pub struct GroundContactDetector {
    last_has_pressure: bool,
    last_time_switched: SystemTime,
    has_ground_contact: bool,
}

#[context]
pub struct CreationContext {
    pub hysteresis: Parameter<f32, "control.ground_contact_detector.hysteresis">,
    pub pressure_threshold: Parameter<f32, "control.ground_contact_detector.pressure_threshold">,
    pub timeout: Parameter<Duration, "control.ground_contact_detector.timeout">,
}

#[context]
pub struct CycleContext {
    pub sensor_data: Input<SensorData, "sensor_data">,
    pub sole_pressure: Input<SolePressure, "sole_pressure">,
    pub cycle_info: Input<CycleInfo, "cycle_info">,

    pub hysteresis: Parameter<f32, "control.ground_contact_detector.hysteresis">,
    pub pressure_threshold: Parameter<f32, "control.ground_contact_detector.pressure_threshold">,
    pub timeout: Parameter<Duration, "control.ground_contact_detector.timeout">,
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
            self.last_time_switched = context.cycle_info.start_time;
        }
        if context
            .cycle_info
            .start_time
            .duration_since(self.last_time_switched)
            .expect("time ran backwards")
            > *context.timeout
        {
            self.has_ground_contact = has_pressure;
        }
        self.last_has_pressure = has_pressure;

        Ok(MainOutputs {
            has_ground_contact: has_pressure.into(),
        })
    }
}
