use booster_sdk::types::RobotMode;
use color_eyre::Result;
use serde::{Deserialize, Serialize};

use context_attribute::context;
use framework::MainOutput;
use hardware::{HighLevelInterface, MotionRuntimeInteface, TimeInterface};
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
pub struct MainOutputs {
    pub robot_mode: MainOutput<Option<RobotMode>>,
}

impl BoosterModeHandler {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(
        &mut self,
        context: CycleContext<impl HighLevelInterface + MotionRuntimeInteface + TimeInterface>,
    ) -> Result<MainOutputs> {
        if context.hardware_interface.get_motion_runtime_type()? != MotionRuntime::Booster {
            return Ok(MainOutputs {
                robot_mode: None.into(),
            });
        }

        let Ok(robot_mode) = context.hardware_interface.get_mode() else {
            return Ok(MainOutputs {
                robot_mode: None.into(),
            });
        };

        match (context.primary_state, robot_mode) {
            (
                PrimaryState::Safe
                | PrimaryState::Stop
                | PrimaryState::Penalized
                | PrimaryState::Initial
                | PrimaryState::Set
                | PrimaryState::Finished,
                RobotMode::Walking,
            ) => change_mode(&context, RobotMode::Prepare),

            (PrimaryState::Ready | PrimaryState::Playing, RobotMode::Prepare) => {
                change_mode(&context, RobotMode::Walking)
            }
            (_, _) => (),
        };

        Ok(MainOutputs {
            robot_mode: Some(robot_mode).into(),
        })
    }
}

fn change_mode(
    context: &CycleContext<impl HighLevelInterface + MotionRuntimeInteface + TimeInterface>,
    robot_mode: RobotMode,
) {
    let _ = context
        .hardware_interface
        .change_mode(robot_mode)
        .inspect_err(|err| log::error!("{err:?}"));
}
