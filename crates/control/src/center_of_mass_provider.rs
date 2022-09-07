use context_attribute::context;
use nalgebra::Point3;

use framework::{MainOutput, RequiredInput};
use types::RobotKinematics;

pub struct CenterOfMassProvider {}

#[context]
pub struct NewContext {}

#[context]
pub struct CycleContext {
    pub robot_kinematics: RequiredInput<RobotKinematics, "robot_kinematics">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub center_of_mass: MainOutput<Point3<f32>>,
}

impl CenterOfMassProvider {
    pub fn new(context: NewContext) -> anyhow::Result<Self> {
        Ok(Self)
    }

    pub fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
