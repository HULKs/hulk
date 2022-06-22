use macros::{module, require_some};

use crate::types::{
    Facing, Joints, Motion, MotionCommand, MotionSafeExits, MotionSelection, MotionType, SensorData,
};

use super::motion_file::{MotionFile, MotionFileInterpolator};

pub struct StandUpFront {
    interpolator: MotionFileInterpolator,
}

#[module(control)]
#[input(path = sensor_data, data_type = SensorData)]
#[input(path = motion_selection, data_type = MotionSelection)]
#[input(path = motion_command, data_type = MotionCommand)]
#[persistent_state(path = motion_safe_exits, data_type = MotionSafeExits)]
#[main_output(name = stand_up_front_positions, data_type = Joints)]
impl StandUpFront {}

impl StandUpFront {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {
            interpolator: MotionFile::from_path("etc/motions/stand_up_front.json")?.into(),
        })
    }

    fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        let last_cycle_duration = require_some!(context.sensor_data)
            .cycle_info
            .last_cycle_duration;
        let motion_selection = require_some!(context.motion_selection);
        let motion_command = require_some!(context.motion_command);

        if motion_selection.current_motion == MotionType::StandUpFront {
            self.interpolator.step(last_cycle_duration);
        } else {
            self.interpolator.reset();
        }

        context.motion_safe_exits[MotionType::StandUpFront] = false;
        if self.interpolator.is_finished() {
            match motion_command.motion {
                Motion::StandUp {
                    facing: Facing::Down,
                } => self.interpolator.reset(),
                _ => {
                    context.motion_safe_exits[MotionType::StandUpFront] = true;
                }
            };
        }

        Ok(MainOutputs {
            stand_up_front_positions: Some(self.interpolator.value()),
        })
    }
}
