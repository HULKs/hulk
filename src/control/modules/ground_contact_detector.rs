use macros::{module, require_some};

use crate::types::SolePressure;

pub struct GroundContactDetector;

#[module(control)]
#[input(path = sole_pressure, data_type = SolePressure)]
#[parameter(path = control.high_detector.total_pressure_threshold, data_type = f32)]
#[main_output(data_type = bool, name = has_ground_contact)]
impl GroundContactDetector {}

impl GroundContactDetector {
    pub fn new() -> Self {
        Self
    }

    fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        let sole_pressure = require_some!(context.sole_pressure);
        let has_ground_contact =
            sole_pressure.left + sole_pressure.right > *context.total_pressure_threshold;
        Ok(MainOutputs {
            has_ground_contact: Some(has_ground_contact),
        })
    }
}
