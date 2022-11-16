use context_attribute::context;
use framework::{MainOutput, Input};
use types::{RobotKinematics, SensorData};

pub struct KinematicsProvider {}

#[context]
pub struct NewContext {}

#[context]
pub struct CycleContext {
    pub sensor_data: Input<SensorData, "sensor_data?">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub robot_kinematics: MainOutput<RobotKinematics>,
}

impl KinematicsProvider {
    pub fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, _context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
