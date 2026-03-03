use color_eyre::Result;
use context_attribute::context;
use hardware::{HighLevelInterface, MotionRuntimeInteface, TimeInterface};
use serde::{Deserialize, Serialize};
use types::{joints::head::HeadJoints, motion_runtime::MotionRuntime};

#[derive(Deserialize, Serialize)]
pub struct RotateHead {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
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
        if matches!(
            context.hardware_interface.get_motion_runtime_type()?,
            MotionRuntime::Booster
        ) {
            context
                .hardware_interface
                .rotate_head(*context.head_joints)?;
        }

        Ok(MainOutputs {})
    }
}
