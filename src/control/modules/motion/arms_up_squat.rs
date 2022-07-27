use module_derive::module;
use types::{Joints, JointsCommand, MotionSafeExits, MotionSelection, MotionType, SensorData};

use super::motion_file::{MotionFile, MotionFileInterpolator};

pub struct ArmsUpSquat {
    interpolator: MotionFileInterpolator,
}

#[module(control)]
#[input(path = sensor_data, data_type = SensorData, required)]
#[input(path = motion_selection, data_type = MotionSelection, required)]
#[persistent_state(path = motion_safe_exits, data_type = MotionSafeExits)]
#[main_output(name = arms_up_squat_joints_command, data_type = JointsCommand)]
impl ArmsUpSquat {}

impl ArmsUpSquat {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {
            interpolator: MotionFile::from_path("etc/motions/arms_up_squat.json")?.into(),
        })
    }

    fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        let last_cycle_duration = context.sensor_data.cycle_info.last_cycle_duration;
        let motion_selection = context.motion_selection;

        if motion_selection.current_motion == MotionType::ArmsUpSquat {
            self.interpolator.step(last_cycle_duration);
        } else {
            self.interpolator.reset();
        }

        Ok(MainOutputs {
            arms_up_squat_joints_command: Some(JointsCommand {
                positions: self.interpolator.value(),
                stiffnesses: Joints::fill(0.9),
            }),
        })
    }
}
