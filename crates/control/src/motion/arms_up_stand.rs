use color_eyre::Result;
use context_attribute::context;
use framework::deserialize_not_implemented;
use framework::MainOutput;
use hardware::PathsInterface;
use motionfile::{InterpolatorState, MotionFile, MotionInterpolator};
use serde::{Deserialize, Serialize};
use types::{
    condition_input::ConditionInput,
    cycle_time::CycleTime,
    joints::Joints,
    motion_selection::{MotionSelection, MotionType},
    motor_commands::MotorCommands,
};

#[derive(Deserialize, Serialize)]
pub struct ArmsUpstand {
    #[serde(skip, default = "deserialize_not_implemented")]
    interpolator: MotionInterpolator<Joints<f32>>,
    state: InterpolatorState<Joints<f32>>,
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
    pub arms_up_stand_joints_command: MainOutput<MotorCommands<Joints<f32>>>,
}

impl ArmsUpstand {
    pub fn new(context: CreationContext<impl PathsInterface>) -> Result<Self> {
        let paths = context.hardware_interface.get_paths();
        Ok(Self {
            interpolator: MotionFile::from_path(paths.motions.join("arms_up_stand.json"))?
                .try_into()?,
            state: InterpolatorState::INITIAL,
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let last_cycle_duration = context.cycle_time.last_cycle_duration;
        let motion_selection = context.motion_selection;
        let condition_input = context.condition_input;

        if motion_selection.current_motion == MotionType::ArmsUpStand {
            self.interpolator
                .advance_state(&mut self.state, last_cycle_duration, condition_input);
        } else {
            self.state.reset();
        }

        Ok(MainOutputs {
            arms_up_stand_joints_command: MotorCommands {
                positions: self.interpolator.value(self.state),
                stiffnesses: Joints::fill(0.9),
            }
            .into(),
        })
    }
}
