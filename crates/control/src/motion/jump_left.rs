use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use types::{
    CycleInfo, JointsCommand, MotionFile, MotionFileInterpolator, MotionSafeExits, MotionSelection,
    MotionType, SensorData, Joints,
};

pub struct JumpLeft {
    interpolator: MotionFileInterpolator,
}

#[context]
pub struct CreationContext {
    pub motion_safe_exits: PersistentState<MotionSafeExits, "motion_safe_exits">,
}

#[context]
pub struct CycleContext {
    pub motion_safe_exits: PersistentState<MotionSafeExits, "motion_safe_exits">,

    pub motion_selection: Input<MotionSelection, "motion_selection">,
    pub sensor_data: Input<SensorData, "sensor_data">,
    pub cycle_info: Input<CycleInfo, "cycle_info">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub jump_left_joints_command: MainOutput<JointsCommand>,
}

impl JumpLeft {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            interpolator: MotionFile::from_path("etc/motions/jump_left.json")?.into(),
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let last_cycle_duration = context.cycle_info.last_cycle_duration;
        if context.motion_selection.current_motion == MotionType::JumpLeft {
            self.interpolator.step(last_cycle_duration);
        } else {
            self.interpolator.reset();
        }

        context.motion_safe_exits[MotionType::JumpLeft] = self.interpolator.is_finished();

        Ok(MainOutputs {
            jump_left_joints_command: JointsCommand {
                positions: self.interpolator.value(),
                stiffnesses: Joints::fill(if self.interpolator.is_finished() {
                    0.0
                } else {
                    0.9
                }),
            }
            .into(),
        })
    }
}
