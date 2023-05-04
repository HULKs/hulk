use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use motionfile::{MotionFile, MotionInterpolator};
use types::{
    ConditionInput, CycleTime, Joints, JointsCommand, MotionFinished, MotionSelection, MotionType,
};

pub struct SitDown {
    interpolator: MotionInterpolator<Joints<f32>>,
}

#[context]
pub struct CreationContext {
    pub motion_finished: PersistentState<MotionFinished, "motion_finished">,
}

#[context]
pub struct CycleContext {
    pub condition_input: Input<ConditionInput, "condition_input">,
    pub cycle_time: Input<CycleTime, "cycle_time">,
    pub motion_selection: Input<MotionSelection, "motion_selection">,

    pub motion_finished: PersistentState<MotionFinished, "motion_finished">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub sit_down_joints_command: MainOutput<JointsCommand<f32>>,
}

impl SitDown {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            interpolator: MotionFile::from_path("etc/motions/sit_down.json")?.try_into()?,
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let last_cycle_duration = context.cycle_time.last_cycle_duration;

        if context.motion_selection.current_motion == MotionType::SitDown {
            self.interpolator
                .advance_by(last_cycle_duration, context.condition_input);
        } else {
            self.interpolator.reset();
        }

        context.motion_finished[MotionType::SitDown] = self.interpolator.is_finished();

        Ok(MainOutputs {
            sit_down_joints_command: JointsCommand {
                positions: self.interpolator.value(),
                stiffnesses: Joints::fill(0.8),
            }
            .into(),
        })
    }
}
