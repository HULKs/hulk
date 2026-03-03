use color_eyre::Result;
use context_attribute::context;
use hardware::{HighLevelInterface, MotionRuntimeInteface, TimeInterface};
use serde::{Deserialize, Serialize};
use types::{motion_command::MotionCommand, motion_runtime::MotionRuntime, step::Step};

#[derive(Deserialize, Serialize)]
pub struct BoosterWalking {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    motion_command: Input<MotionCommand, "motion_command">,

    hardware_interface: HardwareInterface,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {}

impl BoosterWalking {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(
        &mut self,
        context: CycleContext<impl HighLevelInterface + MotionRuntimeInteface + TimeInterface>,
    ) -> Result<MainOutputs> {
        if !matches!(
            context.hardware_interface.get_motion_runtime_type()?,
            MotionRuntime::Booster
        ) {
            return Ok(MainOutputs {});
        }

        #[allow(clippy::single_match)]
        match context.motion_command {
            MotionCommand::WalkWithVelocity {
                velocity,
                angular_velocity,
                ..
            } => context.hardware_interface.move_robot(Step {
                forward: velocity.x(),
                left: velocity.y(),
                turn: *angular_velocity,
            })?,
            _ => (),
        };

        Ok(MainOutputs {})
    }
}
