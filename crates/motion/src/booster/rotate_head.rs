use std::time::{Duration, SystemTime};

use booster_sdk::types::RobotMode;
use color_eyre::Result;
use context_attribute::context;
use hardware::{HighLevelInterface, MotionRuntimeInterface};
use kinematics::joints::head::HeadJoints;
use serde::{Deserialize, Serialize};
use types::{cycle_time::CycleTime, motion_runtime::MotionRuntime};

#[derive(Deserialize, Serialize)]
pub struct RotateHead {
    pub last_rotate_head_time: SystemTime,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    robot_mode: RequiredInput<Option<RobotMode>, "WorldState", "robot_mode?">,

    cycle_time: Input<CycleTime, "cycle_time">,
    head_joints: Input<HeadJoints<f32>, "head_joints_command">,

    rotate_head_message_interval:
        Parameter<Duration, "motion.booster.rotate_head_message_interval">,

    hardware_interface: HardwareInterface,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {}

impl RotateHead {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            last_rotate_head_time: SystemTime::UNIX_EPOCH,
        })
    }

    pub fn cycle(
        &mut self,
        context: CycleContext<impl HighLevelInterface + MotionRuntimeInterface>,
    ) -> Result<MainOutputs> {
        if context.hardware_interface.get_motion_runtime_type()? != MotionRuntime::Booster
            || !matches!(context.robot_mode, RobotMode::Walking)
        {
            return Ok(MainOutputs {});
        }

        if context
            .cycle_time
            .start_time
            .duration_since(self.last_rotate_head_time)
            .expect("Time ran backwards")
            > *context.rotate_head_message_interval
        {
            rotate_head(&context);
        }

        Ok(MainOutputs {})
    }
}

fn rotate_head(context: &CycleContext<impl HighLevelInterface + MotionRuntimeInterface>) {
    let _ = context
        .hardware_interface
        .rotate_head(*context.head_joints)
        .inspect_err(|err| log::error!("{err:?}"));
}
