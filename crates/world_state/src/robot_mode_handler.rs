use booster_sdk::types::RobotMode;
use color_eyre::Result;
use context_attribute::context;
use hardware::{HighLevelInterface, MotionRuntimeInteface, TimeInterface};
use serde::{Deserialize, Serialize};
use types::{motion_runtime::MotionRuntime, primary_state::PrimaryState};

#[derive(Deserialize, Serialize)]
pub struct BoosterModeHandler {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    primary_state: Input<PrimaryState, "primary_state">,

    hardware_interface: HardwareInterface,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {}

impl BoosterModeHandler {
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

        match context.primary_state {
            PrimaryState::Safe
            | PrimaryState::Stop
            | PrimaryState::Penalized
            | PrimaryState::Initial
            | PrimaryState::Set
            | PrimaryState::Finished => {
                context.hardware_interface.change_mode(RobotMode::Prepare)?
            }
            PrimaryState::Ready | PrimaryState::Playing => {
                context.hardware_interface.change_mode(RobotMode::Walking)?;
                context.hardware_interface.enter_wbc_gait()?
            }
        };

        Ok(MainOutputs {})
    }
}
