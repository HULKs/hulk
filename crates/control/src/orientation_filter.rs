use framework::{
    MainOutput, Parameter, OptionalInput
};

pub struct OrientationFilter {}

#[context]
pub struct NewContext {
    pub orientation_filter: Parameter<configuration::OrientationFilter, "control/orientation_filter">,
}

#[context]
pub struct CycleContext {


    pub sensor_data: OptionalInput<SensorData, "sensor_data">,
    pub sole_pressure: OptionalInput<SolePressure, "sole_pressure">,
    pub support_foot: OptionalInput<SupportFoot, "support_foot">,

    pub orientation_filter: Parameter<configuration::OrientationFilter, "control/orientation_filter">,



}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub robot_orientation: MainOutput<UnitComplex<f32>>,
}

impl OrientationFilter {
    pub fn new(context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
