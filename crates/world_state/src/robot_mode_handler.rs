use std::time::{Duration, SystemTime};

use booster_sdk::types::RobotMode;
use color_eyre::Result;
use serde::{Deserialize, Serialize};

use context_attribute::context;
use framework::MainOutput;
use hardware::{HighLevelInterface, MotionRuntimeInterface};
use types::{cycle_time::CycleTime, motion_runtime::MotionRuntime, primary_state::PrimaryState};

#[derive(Deserialize, Serialize)]
pub struct BoosterModeHandler {
    last_primary_state_change_time: SystemTime,
    last_primary_state: PrimaryState,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    primary_state: Input<PrimaryState, "primary_state">,
    cycle_time: Input<CycleTime, "cycle_time">,

    wait_before_prepare: Parameter<Duration, "wait_before_prepare">,

    hardware_interface: HardwareInterface,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub robot_mode: MainOutput<Option<RobotMode>>,
}

impl BoosterModeHandler {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            last_primary_state_change_time: SystemTime::UNIX_EPOCH,
            last_primary_state: PrimaryState::default(),
        })
    }

    pub fn cycle(
        &mut self,
        context: CycleContext<impl HighLevelInterface + MotionRuntimeInterface>,
    ) -> Result<MainOutputs> {
        let motion_robot_mode = match context.hardware_interface.get_motion_runtime_type()? {
            MotionRuntime::Booster => RobotMode::Walking,
            MotionRuntime::Hulk => RobotMode::Custom,
        };

        let Ok(robot_mode) = context.hardware_interface.get_mode() else {
            return Ok(MainOutputs {
                robot_mode: None.into(),
            });
        };

        if context.primary_state != &self.last_primary_state {
            self.last_primary_state_change_time = context.cycle_time.start_time;
            self.last_primary_state = *context.primary_state;
        }
        let time_since_primary_state_change = context
            .cycle_time
            .start_time
            .duration_since(self.last_primary_state_change_time)
            .expect("time ran backwards");
        let switch_to_prepare = &time_since_primary_state_change >= context.wait_before_prepare;

        match (context.primary_state, robot_mode) {
            (
                PrimaryState::Safe | PrimaryState::Initial,
                RobotMode::Walking | RobotMode::Custom,
            ) => change_mode(&context, RobotMode::Prepare),
            (
                PrimaryState::Finished | PrimaryState::Penalized,
                RobotMode::Walking | RobotMode::Custom,
            ) => {
                if switch_to_prepare {
                    change_mode(&context, RobotMode::Prepare)
                }
            }
            (
                PrimaryState::Ready
                | PrimaryState::Playing
                | PrimaryState::Set
                | PrimaryState::Stop,
                RobotMode::Prepare,
            ) => change_mode(&context, motion_robot_mode),
            (_, _) => (),
        };

        Ok(MainOutputs {
            robot_mode: Some(robot_mode).into(),
        })
    }
}

fn change_mode(
    context: &CycleContext<impl HighLevelInterface + MotionRuntimeInterface>,
    robot_mode: RobotMode,
) {
    let _ = context
        .hardware_interface
        .change_mode(robot_mode)
        .inspect_err(|err| log::error!("{err:?}"));
}
