use macros::{module, require_some};

use crate::types::{
    BodyMotionSafeExits, BodyMotionType, Facing, Motion, MotionCommand, MotionSelection,
    SensorData, StandUpFrontPositions,
};

use super::motion_file::{MotionFile, MotionFileInterpolator};

pub struct StandUpFront {
    interpolator: MotionFileInterpolator,
}

#[module(control)]
#[input(path = sensor_data, data_type = SensorData)]
#[input(path = motion_selection, data_type = MotionSelection)]
#[input(path = motion_command, data_type = MotionCommand)]
#[persistent_state(path = body_motion_safe_exits, data_type = BodyMotionSafeExits)]
#[main_output(data_type = StandUpFrontPositions)]
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

        if motion_selection.current_body_motion == BodyMotionType::StandUpFront {
            self.interpolator.step(last_cycle_duration);
        } else {
            self.interpolator.reset();
        }

        context.body_motion_safe_exits[BodyMotionType::StandUpFront] = false;
        if self.interpolator.is_finished() {
            match motion_command.motion {
                Motion::StandUp {
                    facing: Facing::Down,
                } => self.interpolator.reset(),
                _ => context.body_motion_safe_exits[BodyMotionType::StandUpFront] = true,
            };
        }

        Ok(MainOutputs {
            stand_up_front_positions: Some(StandUpFrontPositions {
                positions: self.interpolator.value().into(),
            }),
        })
    }
}
