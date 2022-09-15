use framework::{
    MainOutput, Parameter, OptionalInput
};

pub struct GroundContactDetector {}

#[context]
pub struct NewContext {
    pub hysteresis: Parameter<f32, "control/ground_contact_detector/hysteresis">,
    pub pressure_threshold: Parameter<f32, "control/ground_contact_detector/pressure_threshold">,
    pub timeout: Parameter<Duration, "control/ground_contact_detector/timeout">,
}

#[context]
pub struct CycleContext {


    pub sensor_data: OptionalInput<SensorData, "sensor_data">,
    pub sole_pressure: OptionalInput<SolePressure, "sole_pressure">,

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
    pub fn new(context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
