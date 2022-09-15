use context_attribute::context;
use framework::{MainOutput, OptionalInput};

pub struct GroundProvider {}

#[context]
pub struct NewContext {}

#[context]
pub struct CycleContext {
    pub robot_kinematics: OptionalInput<RobotKinematics, "robot_kinematics">,
    pub sensor_data: OptionalInput<SensorData, "sensor_data">,
    pub support_foot: OptionalInput<SupportFoot, "support_foot">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub robot_to_ground: MainOutput<Isometry3<f32>>,
    pub ground_to_robot: MainOutput<Isometry3<f32>>,
}

impl GroundProvider {
    pub fn new(context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
