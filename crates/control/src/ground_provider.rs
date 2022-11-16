use context_attribute::context;
use framework::{Input, MainOutput, RequiredInput};
use nalgebra::Isometry3;
use types::{RobotKinematics, SensorData, SupportFoot};

pub struct GroundProvider {}

#[context]
pub struct NewContext {}

#[context]
pub struct CycleContext {
    pub robot_kinematics: Input<RobotKinematics, "robot_kinematics">,
    pub sensor_data: Input<SensorData, "sensor_data">,
    pub support_foot: RequiredInput<Option<SupportFoot>, "support_foot?">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub robot_to_ground: MainOutput<Option<Isometry3<f32>>>,
    pub ground_to_robot: MainOutput<Option<Isometry3<f32>>>,
}

impl GroundProvider {
    pub fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, _context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
