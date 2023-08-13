use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use hardware::PathsInterface;
use motionfile::{MotionFile, MotionInterpolator};
use serde::{Deserialize, Serialize};
use types::{
    condition_input::ConditionInput,
    cycle_time::CycleTime,
    joints::JointsCommand,
    motion_selection::{MotionSafeExits, MotionSelection, MotionType},
};

#[derive(Deserialize, Serialize)]
pub struct JumpRight {
    interpolator: MotionInterpolator<JointsCommand<f32>>,
}

#[context]
pub struct CreationContext {
    hardware_interface: HardwareInterface,
}

#[context]
pub struct CycleContext {
    motion_safe_exits: CyclerState<MotionSafeExits, "motion_safe_exits">,

    condition_input: Input<ConditionInput, "condition_input">,
    cycle_time: Input<CycleTime, "cycle_time">,
    motion_selection: Input<MotionSelection, "motion_selection">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub jump_right_joints_command: MainOutput<JointsCommand<f32>>,
}

impl JumpRight {
    pub fn new(context: CreationContext<impl PathsInterface>) -> Result<Self> {
        let paths = context.hardware_interface.get_paths();
        Ok(Self {
            interpolator: MotionFile::from_path(paths.motions.join("jump_left.json"))?
                .try_into()?,
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let last_cycle_duration = context.cycle_time.last_cycle_duration;
        if context.motion_selection.current_motion == MotionType::JumpRight {
            self.interpolator
                .advance_by(last_cycle_duration, context.condition_input);
        } else {
            self.interpolator.reset();
        }

        context.motion_safe_exits[MotionType::JumpRight] = self.interpolator.is_finished();

        Ok(MainOutputs {
            jump_right_joints_command: self.interpolator.value().mirrored().into(),
        })
    }
}
