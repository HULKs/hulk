use context_attribute::context;
use framework::{AdditionalOutput, MainOutput, OptionalInput};

pub struct Odometry {}

#[context]
pub struct NewContext {}

#[context]
pub struct CycleContext {
    pub accumulated_odometry: AdditionalOutput<Isometry2<f32>, "accumulated_odometry">,

    pub robot_kinematics: OptionalInput<RobotKinematics, "robot_kinematics?">,
    pub robot_orientation: OptionalInput<UnitComplex<f32>, "robot_orientation?">,
    pub support_foot: OptionalInput<SupportFoot, "support_foot?">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub current_odometry_to_last_odometry: MainOutput<Isometry2<f32>>,
}

impl Odometry {
    pub fn new(context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
