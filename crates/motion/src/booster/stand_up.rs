use booster_sdk::types::RobotMode;
use color_eyre::Result;
use context_attribute::context;
use hardware::{HighLevelInterface, MotionRuntimeInteface, TimeInterface};
use serde::{Deserialize, Serialize};
use types::{motion_command::MotionCommand, motion_runtime::MotionRuntime};

#[derive(Deserialize, Serialize)]
pub struct BoosterStandUp {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    robot_mode: RequiredInput<Option<RobotMode>, "WorldState", "robot_mode?">,

    motion_command: Input<MotionCommand, "WorldState", "motion_command">,

    hardware_interface: HardwareInterface,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {}

impl BoosterStandUp {
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
        ) | !matches!(context.robot_mode, RobotMode::Prepare | RobotMode::Damping)
        {
            return Ok(MainOutputs {});
        }

        if matches!(context.motion_command, MotionCommand::StandUp) {
            let _ = context
                .hardware_interface
                .get_up()
                .inspect_err(|err| log::error!("{err:?}"));
        };

        Ok(MainOutputs {})
    }
}
