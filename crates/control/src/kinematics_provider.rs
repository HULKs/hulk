use context_attribute::context;
use framework::{MainOutput, OptionalInput};

pub struct KinematicsProvider {}

#[context]
pub struct NewContext {}

#[context]
pub struct CycleContext {
    pub sensor_data: OptionalInput<SensorData, "sensor_data">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub robot_kinematics: MainOutput<RobotKinematics>,
}

impl KinematicsProvider {
    pub fn new(context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
