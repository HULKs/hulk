use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use types::{RobotKinematics, SensorData};

pub struct KinematicsProvider {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    pub sensor_data: Input<SensorData, "sensor_data">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub robot_kinematics: MainOutput<RobotKinematics>,
}

impl KinematicsProvider {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, _context: CycleContext) -> Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
