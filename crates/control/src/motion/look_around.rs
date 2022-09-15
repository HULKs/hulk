use framework::{
    RequiredInput, MainOutput, Parameter
};

pub struct LookAround {}

#[context]
pub struct NewContext {
    pub config: Parameter<configuration::LookAround, "control/look_around">,
}

#[context]
pub struct CycleContext {



    pub config: Parameter<configuration::LookAround, "control/look_around">,



    pub motion_command: RequiredInput<MotionCommand, "motion_command">,
    pub sensor_data: RequiredInput<SensorData, "sensor_data">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub look_around: MainOutput<HeadJoints>,
}

impl LookAround {
    pub fn new(context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
