use context_attribute::context;
use framework::{AdditionalOutput, MainOutput, Input};
use nalgebra::{Isometry2, UnitComplex};
use types::{RobotKinematics, SupportFoot};

pub struct Odometry {}

#[context]
pub struct NewContext {}

#[context]
pub struct CycleContext {
    pub accumulated_odometry: AdditionalOutput<Isometry2<f32>, "accumulated_odometry">,

    pub robot_kinematics: Input<RobotKinematics, "robot_kinematics?">,
    pub robot_orientation: Input<UnitComplex<f32>, "robot_orientation?">,
    pub support_foot: Input<SupportFoot, "support_foot?">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub current_odometry_to_last_odometry: MainOutput<Isometry2<f32>>,
}

impl Odometry {
    pub fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, _context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
