use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use hardware::PathsInterface;
use motionfile::{MotionFile, MotionInterpolator};
use serde::{Deserialize, Serialize};
use types::{
    condition_input::ConditionInput,
    cycle_time::CycleTime,
    joints::Joints,
    motion_selection::{MotionSelection, MotionType},
    motor_command::MotorCommand,
};

#[derive(Deserialize, Serialize)]
pub struct ArmsUpSquat {
    interpolator: MotionInterpolator<Joints<f32>>,
}

#[context]
pub struct CreationContext {
    hardware_interface: HardwareInterface,
}

#[context]
pub struct CycleContext {
    condition_input: Input<ConditionInput, "condition_input">,
    motion_selection: Input<MotionSelection, "motion_selection">,
    cycle_time: Input<CycleTime, "cycle_time">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub arms_up_squat_joints_command: MainOutput<MotorCommand<f32>>,
}

impl ArmsUpSquat {
    pub fn new(context: CreationContext<impl PathsInterface>) -> Result<Self> {
        let paths = context.hardware_interface.get_paths();
        Ok(Self {
            interpolator: MotionFile::from_path(paths.motions.join("arms_up_squat.json"))?
                .try_into()?,
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let last_cycle_duration = context.cycle_time.last_cycle_duration;
        let motion_selection = context.motion_selection;

        if motion_selection.current_motion == MotionType::ArmsUpSquat {
            self.interpolator
                .advance_by(last_cycle_duration, context.condition_input);
        } else {
            self.interpolator.reset();
        }

        Ok(MainOutputs {
            arms_up_squat_joints_command: MotorCommand {
                positions: self.interpolator.value(),
                stiffnesses: Joints::fill(0.9),
            }
            .into(),
        })
    }
}
