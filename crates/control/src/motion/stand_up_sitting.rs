use std::time::Duration;

use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use hardware::PathsInterface;
use motionfile::{MotionFile, MotionInterpolator};
use serde::{Deserialize, Serialize};
use types::{
    condition_input::ConditionInput,
    cycle_time::CycleTime,
    joints::Joints,
    motion_selection::{MotionSafeExits, MotionSelection, MotionType},
};

#[derive(Deserialize, Serialize)]
pub struct StandUpSitting {
    interpolator: MotionInterpolator<Joints<f32>>,
}

#[context]
pub struct CreationContext {
    hardware_interface: HardwareInterface,
}

#[context]
pub struct CycleContext {
    condition_input: Input<ConditionInput, "condition_input">,
    cycle_time: Input<CycleTime, "cycle_time">,
    motion_selection: Input<MotionSelection, "motion_selection">,

    motion_safe_exits: CyclerState<MotionSafeExits, "motion_safe_exits">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub stand_up_sitting_positions: MainOutput<Joints<f32>>,
    pub stand_up_sitting_estimated_remaining_duration: MainOutput<Option<Duration>>,
}

impl StandUpSitting {
    pub fn new(context: CreationContext<impl PathsInterface>) -> Result<Self> {
        let paths = context.hardware_interface.get_paths();
        Ok(Self {
            interpolator: MotionFile::from_path(paths.motions.join("stand_up_sitting.json"))?
                .try_into()?,
        })
    }

    pub fn advance_interpolator(&mut self, context: CycleContext) {
        let last_cycle_duration = context.cycle_time.last_cycle_duration;
        let condition_input = context.condition_input;

        context.motion_safe_exits[MotionType::StandUpSitting] = false;

        self.interpolator
            .advance_by(last_cycle_duration, condition_input);

        if self.interpolator.is_finished() {
            context.motion_safe_exits[MotionType::StandUpSitting] = true;
        }
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let estimated_remaining_duration =
            if let MotionType::StandUpSitting = context.motion_selection.current_motion {
                self.advance_interpolator(context);
                Some(self.interpolator.estimated_remaining_duration())
            } else {
                self.interpolator.reset();
                None
            };
        Ok(MainOutputs {
            stand_up_sitting_positions: self.interpolator.value().into(),
            stand_up_sitting_estimated_remaining_duration: estimated_remaining_duration.into(),
        })
    }
}
