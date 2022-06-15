use macros::{module, require_some};

use crate::{
    control::filtering::Hysteresis,
    types::{GroundContact, SolePressure},
};

pub struct GroundContactDetector {
    left_hysteresis: Hysteresis,
    right_hysteresis: Hysteresis,
}

#[module(control)]
#[input(path = sole_pressure, data_type = SolePressure)]
#[parameter(path = control.ground_contact_detector.pressure_threshold, data_type = f32)]
#[parameter(path = control.ground_contact_detector.hysteresis, data_type = f32)]
#[main_output(data_type = GroundContact, name = ground_contact)]
impl GroundContactDetector {}

impl GroundContactDetector {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {
            left_hysteresis: Hysteresis::new(),
            right_hysteresis: Hysteresis::new(),
        })
    }

    fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        let sole_pressure = require_some!(context.sole_pressure);
        let left_foot = self.left_hysteresis.update_greater_than(
            sole_pressure.left,
            *context.pressure_threshold,
            *context.hysteresis,
        );
        let right_foot = self.right_hysteresis.update_greater_than(
            sole_pressure.right,
            *context.pressure_threshold,
            *context.hysteresis,
        );
        Ok(MainOutputs {
            ground_contact: Some(GroundContact {
                left_foot,
                right_foot,
            }),
        })
    }
}
