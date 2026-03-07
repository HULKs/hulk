use booster_sdk::types::RobotMode;
use color_eyre::Result;
use context_attribute::context;
use hardware::{HighLevelInterface, MotionRuntimeInteface, TimeInterface};
use kinematics::joints::head::HeadJoints;
use serde::{Deserialize, Serialize};
use types::motion_runtime::MotionRuntime;

#[derive(Deserialize, Serialize)]
pub struct RotateHead {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    robot_mode: RequiredInput<Option<RobotMode>, "WorldState", "robot_mode?">,

    head_joints: Input<HeadJoints<f32>, "head_joints_command">,

    hardware_interface: HardwareInterface,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {}

impl RotateHead {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(
        &mut self,
        context: CycleContext<impl HighLevelInterface + MotionRuntimeInteface + TimeInterface>,
    ) -> Result<MainOutputs> {
        if context.hardware_interface.get_motion_runtime_type()? != MotionRuntime::Booster
            || !matches!(context.robot_mode, RobotMode::Walking)
        {
            return Ok(MainOutputs {});
        }

        rotate_head(&context);

        Ok(MainOutputs {})
    }
}

fn rotate_head(
    context: &CycleContext<impl HighLevelInterface + MotionRuntimeInteface + TimeInterface>,
) {
    let _ = context
        .hardware_interface
        .rotate_head(*context.head_joints)
        .inspect_err(|err| log::error!("{err:?}"));
}
