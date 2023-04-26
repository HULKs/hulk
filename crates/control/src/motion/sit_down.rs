use color_eyre::Result;
use context_attribute::context;
use framework::{MainOutput, AdditionalOutput};
use motionfile::{MotionFile, MotionInterpolator};
use types::{
    ConditionInput, CycleTime, Joints, JointsCommand, MotionSafeExits, MotionSelection, MotionType,
};

pub struct SitDown {
    interpolator: MotionInterpolator<Joints<f32>>,
}

#[context]
pub struct CreationContext {
    pub motion_safe_exits: PersistentState<MotionSafeExits, "motion_safe_exits">,
}

#[context]
pub struct CycleContext {
    pub condition_input: Input<ConditionInput, "condition_input">,
    pub cycle_time: Input<CycleTime, "cycle_time">,
    pub motion_selection: Input<MotionSelection, "motion_selection">,

    pub motion_safe_exits: PersistentState<MotionSafeExits, "motion_safe_exits">,

    pub time_since_start: AdditionalOutput<f32, "time_since_start">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub sit_down_joints_command: MainOutput<JointsCommand<f32>>,
}

impl SitDown {
    pub fn new(_context: CreationContext) -> Result<Self> {
        let motionfile = MotionFile::from_path("etc/motions/sit_down.json")?;
        let interpolator = motionfile.try_into()?;
        
        Ok(Self {
            interpolator
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let last_cycle_duration = context.cycle_time.last_cycle_duration;

        if context.motion_selection.current_motion == MotionType::SitDown {
            self.interpolator
                .advance_by(last_cycle_duration, context.condition_input);
        } else {
            self.interpolator.reset();
        }

        context.motion_safe_exits[MotionType::SitDown] = self.interpolator.is_finished();
        context.time_since_start.fill_if_subscribed(|| self.interpolator.current_time().as_secs_f32());

        Ok(MainOutputs {
            sit_down_joints_command: JointsCommand {
                positions: self.interpolator.value(),
                stiffnesses: Joints::fill(0.8),
            }
            .into(),
        })
    }
}
