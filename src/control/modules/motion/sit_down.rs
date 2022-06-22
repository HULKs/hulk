use macros::{module, require_some};

use crate::types::{
    Joints, MotionSafeExits, MotionSelection, MotionType, SensorData, SitDownJoints,
};

use super::motion_file::{MotionFile, MotionFileInterpolator};

pub struct SitDown {
    interpolator: MotionFileInterpolator,
}

#[module(control)]
#[input(path = sensor_data, data_type = SensorData)]
#[input(path = motion_selection, data_type = MotionSelection)]
#[persistent_state(path = motion_safe_exits, data_type = MotionSafeExits)]
#[main_output(data_type = SitDownJoints)]
impl SitDown {}

impl SitDown {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {
            interpolator: MotionFile::from_path("etc/motions/sit_down.json")?.into(),
        })
    }

    fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        let last_cycle_duration = require_some!(context.sensor_data)
            .cycle_info
            .last_cycle_duration;
        let motion_selection = require_some!(context.motion_selection);

        if motion_selection.current_motion == MotionType::SitDown {
            self.interpolator.step(last_cycle_duration);
        } else {
            self.interpolator.reset();
        }

        context.motion_safe_exits[MotionType::SitDown] = self.interpolator.is_finished();

        Ok(MainOutputs {
            sit_down_joints: Some(SitDownJoints {
                positions: self.interpolator.value(),
                stiffnesses: Joints::fill(if self.interpolator.is_finished() {
                    0.0
                } else {
                    0.8
                }),
            }),
        })
    }
}
