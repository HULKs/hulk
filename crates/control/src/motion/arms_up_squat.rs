use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use types::{
    hardware::Interface, CycleTime, Joints, JointsCommand, MotionFile, MotionFileInterpolator,
    MotionSafeExits, MotionSelection, MotionType, SensorData,
};

pub struct ArmsUpSquat {
    interpolator: MotionFileInterpolator,
}

#[context]
pub struct CreationContext {
    pub hardware_interface: HardwareInterface,

    pub motion_safe_exits: PersistentState<MotionSafeExits, "motion_safe_exits">,
}

#[context]
pub struct CycleContext {
    pub motion_safe_exits: PersistentState<MotionSafeExits, "motion_safe_exits">,

    pub motion_selection: Input<MotionSelection, "motion_selection">,
    pub sensor_data: Input<SensorData, "sensor_data">,
    pub cycle_time: Input<CycleTime, "cycle_time">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub arms_up_squat_joints_command: MainOutput<JointsCommand>,
}

impl ArmsUpSquat {
    pub fn new(context: CreationContext<impl Interface>) -> Result<Self> {
        let paths = context.hardware_interface.get_paths();
        Ok(Self {
            interpolator: MotionFile::from_path(paths.motions.join("arms_up_squat.json"))?.into(),
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let last_cycle_duration = context.cycle_time.last_cycle_duration;
        let motion_selection = context.motion_selection;

        if motion_selection.current_motion == MotionType::ArmsUpSquat {
            self.interpolator.step(last_cycle_duration);
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
