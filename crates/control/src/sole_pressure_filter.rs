use color_eyre::Result;
use context_attribute::context;
use filtering::LowPassFilter;
use framework::MainOutput;
use types::{SensorData, SolePressure};

pub struct SolePressureFilter {
    left_sole_pressure: LowPassFilter<f32>,
    right_sole_pressure: LowPassFilter<f32>,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    pub sensor_data: Input<SensorData, "sensor_data">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub sole_pressure: MainOutput<SolePressure>,
}

impl SolePressureFilter {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            left_sole_pressure: LowPassFilter::with_alpha(0.0, 0.5),
            right_sole_pressure: LowPassFilter::with_alpha(0.0, 0.5),
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let force_sensitive_resistors = &context.sensor_data.force_sensitive_resistors;
        let left_sole_pressure = force_sensitive_resistors.left.sum();
        self.left_sole_pressure.update(left_sole_pressure);
        let right_sole_pressure = force_sensitive_resistors.right.sum();
        self.right_sole_pressure.update(right_sole_pressure);
        Ok(MainOutputs::default())
    }
}
