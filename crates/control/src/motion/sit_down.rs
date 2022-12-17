use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use types::{
    CycleTime, Joints, JointsCommand, MotionFile, MotionFileInterpolator, MotionSafeExits,
    MotionSelection, MotionType, SensorData,
};

pub struct SitDown {
    interpolator: MotionFileInterpolator,
}

#[context]
pub struct CreationContext {
    pub motion_safe_exits: PersistentState<MotionSafeExits, "motion_safe_exits">,
}

#[context]
pub struct CycleContext {
    pub motion_selection: Input<MotionSelection, "motion_selection">,
    pub sensor_data: Input<SensorData, "sensor_data">,
    pub cycle_time: Input<CycleTime, "cycle_time">,

    pub motion_safe_exits: PersistentState<MotionSafeExits, "motion_safe_exits">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub sit_down_joints_command: MainOutput<JointsCommand>,
}

impl SitDown {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            interpolator: MotionFile::from_path("etc/motions/sit_down.json")?.into(),
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let last_cycle_duration = context.cycle_time.last_cycle_duration;
        if context.motion_selection.current_motion == MotionType::SitDown {
            self.interpolator.step(last_cycle_duration);
        } else {
            self.interpolator.reset();
        }

        context.motion_safe_exits[MotionType::SitDown] = self.interpolator.is_finished();

        Ok(MainOutputs {
            sit_down_joints_command: JointsCommand {
                positions: self.interpolator.value(),
                stiffnesses: Joints::fill(if self.interpolator.is_finished() {
                    0.0
                } else {
                    0.8
                }),
            }
            .into(),
        })
    }
}
