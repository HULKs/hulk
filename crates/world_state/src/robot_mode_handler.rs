use std::time::{Duration, SystemTime};

use booster_sdk::types::RobotMode;
use color_eyre::Result;
use serde::{Deserialize, Serialize};

use context_attribute::context;
use framework::MainOutput;
use hardware::{HighLevelInterface, MotionRuntimeInterface};
use types::{
    buttons::{ButtonPressType, Buttons},
    cycle_time::CycleTime,
    motion_runtime::MotionRuntime,
    primary_state::PrimaryState,
};

#[derive(Deserialize, Serialize)]
pub struct BoosterModeHandler {
    last_primary_state_change_time: SystemTime,
    last_primary_state: PrimaryState,
    local_stop_toggle: bool,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    primary_state: Input<PrimaryState, "primary_state">,
    cycle_time: Input<CycleTime, "cycle_time">,
    buttons: Input<Buttons<Option<ButtonPressType>>, "buttons">,

    wait_before_prepare: Parameter<Duration, "wait_before_prepare">,
    remote_stop_toggle: Parameter<bool, "remote_stop_toggle">,

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
            local_stop_toggle: false,
        })
    }

    pub fn cycle(
        &mut self,
        context: CycleContext<impl HighLevelInterface + MotionRuntimeInterface>,
    ) -> Result<MainOutputs> {
        if context.hardware_interface.get_motion_runtime_type()? != MotionRuntime::Booster {
            return Ok(MainOutputs {
                robot_mode: None.into(),
            });
        }

        let is_local_stop_toggle_short_press =
            matches!(context.buttons.f1, Some(ButtonPressType::Short))
                || matches!(context.buttons.stand, Some(ButtonPressType::Short));

        let should_enter_damping_mode = self.local_stop_toggle != *context.remote_stop_toggle;

        if should_enter_damping_mode && is_local_stop_toggle_short_press {
            self.local_stop_toggle = !self.local_stop_toggle;
        }

        if should_enter_damping_mode {
            change_mode(&context, RobotMode::Damping);
            return Ok(MainOutputs {
                robot_mode: Some(RobotMode::Damping).into(),
            });
        }

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

        match (context.primary_state, robot_mode, switch_to_prepare) {
            (PrimaryState::Safe | PrimaryState::Initial, RobotMode::Walking, _) => {
                change_mode(&context, RobotMode::Prepare)
            }
            (PrimaryState::Finished | PrimaryState::Penalized, RobotMode::Walking, true) => {
                change_mode(&context, RobotMode::Prepare)
            }
            (
                PrimaryState::Ready
                | PrimaryState::Playing
                | PrimaryState::Set
                | PrimaryState::Stop,
                RobotMode::Prepare,
                _,
            ) => change_mode(&context, RobotMode::Walking),
            (_, _, _) => (),
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
