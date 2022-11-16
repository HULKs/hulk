use context_attribute::context;
use framework::{MainOutput, Input};
use nalgebra::Point3;
use types::RobotKinematics;

pub struct CenterOfMassProvider {}

#[context]
pub struct NewContext {}

#[context]
pub struct CycleContext {
    pub robot_kinematics: Input<RobotKinematics, "robot_kinematics">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub center_of_mass: MainOutput<Point3<f32>>,
}

impl CenterOfMassProvider {
    pub fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, _context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
