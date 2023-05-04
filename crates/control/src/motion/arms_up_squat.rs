use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use motionfile::{MotionFile, MotionInterpolator};
use types::{
    ConditionInput, CycleTime, Joints, JointsCommand, MotionFinished, MotionSelection, MotionType,
    SensorData,
};

pub struct ArmsUpSquat {
    interpolator: MotionInterpolator<Joints<f32>>,
}

#[context]
pub struct CreationContext {
    pub motion_finished: PersistentState<MotionFinished, "motion_finished">,
}

#[context]
pub struct CycleContext {
    pub motion_finished: PersistentState<MotionFinished, "motion_finished">,

    pub condition_input: Input<ConditionInput, "condition_input">,
    pub motion_selection: Input<MotionSelection, "motion_selection">,
    pub sensor_data: Input<SensorData, "sensor_data">,
    pub cycle_time: Input<CycleTime, "cycle_time">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub arms_up_squat_joints_command: MainOutput<JointsCommand<f32>>,
}

impl ArmsUpSquat {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            interpolator: MotionFile::from_path("etc/motions/arms_up_squat.json")?.try_into()?,
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
            arms_up_squat_joints_command: JointsCommand {
                positions: self.interpolator.value(),
                stiffnesses: Joints::fill(0.9),
            }
            .into(),
        })
    }
}
