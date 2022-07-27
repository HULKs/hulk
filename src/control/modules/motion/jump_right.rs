use module_derive::module;
use types::{Joints, JointsCommand, MotionSafeExits, MotionSelection, MotionType, SensorData};

use super::motion_file::{MotionFile, MotionFileInterpolator};

pub struct JumpRight {
    interpolator: MotionFileInterpolator,
}

#[module(control)]
#[input(path = sensor_data, data_type = SensorData, required)]
#[input(path = motion_selection, data_type = MotionSelection, required)]
#[persistent_state(path = motion_safe_exits, data_type = MotionSafeExits)]
#[main_output(name = jump_right_joints_command, data_type = JointsCommand)]
impl JumpRight {}

impl JumpRight {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {
            interpolator: MotionFile::from_path("etc/motions/jump_left.json")?.into(),
        })
    }

    fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        let last_cycle_duration = context.sensor_data.cycle_info.last_cycle_duration;
        let motion_selection = context.motion_selection;

        if motion_selection.current_motion == MotionType::JumpRight {
            self.interpolator.step(last_cycle_duration);
        } else {
            self.interpolator.reset();
        }

        context.motion_safe_exits[MotionType::JumpRight] = self.interpolator.is_finished();

        Ok(MainOutputs {
            jump_right_joints_command: Some(JointsCommand {
                positions: self.interpolator.value().mirrored(),
                stiffnesses: Joints::fill(if self.interpolator.is_finished() {
                    0.0
                } else {
                    0.9
                }),
            }),
        })
    }
}
