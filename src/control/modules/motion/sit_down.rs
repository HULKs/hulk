use macros::{module, require_some};

use crate::types::{
    BodyJoints, BodyMotionSafeExits, BodyMotionType, MotionSelection, SensorData, SitDownPositions,
};

use super::motion_file::{MotionFile, MotionFileInterpolator};

pub struct SitDown {
    interpolator: MotionFileInterpolator,
}

#[module(control)]
#[input(path = sensor_data, data_type = SensorData)]
#[input(path = motion_selection, data_type = MotionSelection)]
#[persistent_state(path = body_motion_safe_exits, data_type = BodyMotionSafeExits)]
#[main_output(data_type = SitDownPositions)]
impl SitDown {}

impl SitDown {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            interpolator: MotionFile::from_path("etc/motions/sit_down.json")?.into(),
        })
    }

    fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        let last_cycle_duration = require_some!(context.sensor_data)
            .cycle_info
            .last_cycle_duration;
        let motion_selection = require_some!(context.motion_selection);

        if motion_selection.current_body_motion == BodyMotionType::SitDown {
            self.interpolator.step(last_cycle_duration);
        } else {
            self.interpolator.reset();
        }

        context.body_motion_safe_exits[BodyMotionType::SitDown] = self.interpolator.is_finished();

        Ok(MainOutputs {
            sit_down_positions: Some(SitDownPositions {
                positions: BodyJoints::from(self.interpolator.value()),
                stiffnesses: BodyJoints::fill(if self.interpolator.is_finished() {
                    0.0
                } else {
                    0.8
                }),
            }),
        })
    }
}
