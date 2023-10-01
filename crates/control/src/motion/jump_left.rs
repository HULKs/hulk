use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use hardware::PathsInterface;
use motionfile::{MotionFile, MotionInterpolator};
use types::{
    condition_input::ConditionInput,
    cycle_time::CycleTime,
    joints::JointsCommand,
    motion_selection::{MotionSafeExits, MotionSelection, MotionType},
    sensor_data::SensorData,
};

pub struct JumpLeft {
    interpolator: MotionInterpolator<JointsCommand<f32>>,
}

#[context]
pub struct CreationContext {
    pub hardware_interface: HardwareInterface,
    pub motion_safe_exits: PersistentState<MotionSafeExits, "motion_safe_exits">,
}

#[context]
pub struct CycleContext {
    pub motion_safe_exits: PersistentState<MotionSafeExits, "motion_safe_exits">,

    pub condition_input: Input<ConditionInput, "condition_input">,
    pub cycle_time: Input<CycleTime, "cycle_time">,
    pub motion_selection: Input<MotionSelection, "motion_selection">,
    pub sensor_data: Input<SensorData, "sensor_data">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub jump_left_joints_command: MainOutput<JointsCommand<f32>>,
}

impl JumpLeft {
    pub fn new(context: CreationContext<impl PathsInterface>) -> Result<Self> {
        let paths = context.hardware_interface.get_paths();
        Ok(Self {
            interpolator: MotionFile::from_path(paths.motions.join("jump_left.json"))?
                .try_into()?,
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let last_cycle_duration = context.cycle_time.last_cycle_duration;
        if context.motion_selection.current_motion == MotionType::JumpLeft {
            self.interpolator
                .advance_by(last_cycle_duration, context.condition_input);
        } else {
            self.interpolator.reset();
        }

        context.motion_safe_exits[MotionType::JumpLeft] = self.interpolator.is_finished();

        Ok(MainOutputs {
            jump_left_joints_command: self.interpolator.value().into(),
        })
    }
}
