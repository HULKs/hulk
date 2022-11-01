use context_attribute::context;
use framework::{MainOutput, OptionalInput, Parameter};
use nalgebra::UnitComplex;
use types::{
    configuration::OrientationFilter as OrientationFilterConfiguration, SensorData, SolePressure,
    SupportFoot,
};

pub struct OrientationFilter {}

#[context]
pub struct NewContext {
    pub orientation_filter: Parameter<OrientationFilterConfiguration, "control/orientation_filter">,
}

#[context]
pub struct CycleContext {
    pub sensor_data: OptionalInput<SensorData, "sensor_data?">,
    pub sole_pressure: OptionalInput<SolePressure, "sole_pressure?">,
    pub support_foot: OptionalInput<SupportFoot, "support_foot?">,

    pub orientation_filter: Parameter<OrientationFilterConfiguration, "control/orientation_filter">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub robot_orientation: MainOutput<UnitComplex<f32>>,
}

impl OrientationFilter {
    pub fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, _context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
