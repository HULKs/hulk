use macros::{module, require_some};

use crate::{
    control::filtering::LowPassFilter,
    types::{SensorData, SolePressure},
};

pub struct SolePressureFilter {
    left_sole_pressure: LowPassFilter<f32>,
    right_sole_pressure: LowPassFilter<f32>,
}

#[module(control)]
#[input(path = sensor_data, data_type = SensorData)]
#[main_output(data_type = SolePressure)]
impl SolePressureFilter {}

impl SolePressureFilter {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {
            left_sole_pressure: LowPassFilter::with_alpha(0.0, 0.5),
            right_sole_pressure: LowPassFilter::with_alpha(0.0, 0.5),
        })
    }

    fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        let force_sensitive_resistors =
            &require_some!(context.sensor_data).force_sensitive_resistors;

        let left_sole_pressure = force_sensitive_resistors.left.sum();
        self.left_sole_pressure.update(left_sole_pressure);
        let right_sole_pressure = force_sensitive_resistors.right.sum();
        self.right_sole_pressure.update(right_sole_pressure);

        Ok(MainOutputs {
            sole_pressure: Some(SolePressure {
                left: self.left_sole_pressure.state(),
                right: self.right_sole_pressure.state(),
            }),
        })
    }
}
