use std::time::Duration;

use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use hardware::PathsInterface;
use motionfile::{MotionFile, MotionInterpolator};
use types::ConditionInput;
use types::{CycleTime, Joints, MotionSafeExits, MotionSelection, MotionType};

pub struct StandUpBack {
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

    motion_safe_exits: PersistentState<MotionSafeExits, "motion_safe_exits">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub stand_up_back_positions: MainOutput<Joints<f32>>,
    pub stand_up_back_estimated_remaining_duration: MainOutput<Option<Duration>>,
}

impl StandUpBack {
    pub fn new(context: CreationContext<impl PathsInterface>) -> Result<Self> {
        let paths = context.hardware_interface.get_paths();
        Ok(Self {
            interpolator: MotionFile::from_path(
                paths.motions.join("stand_up_back_dortmund_2022.json"),
            )?
            .try_into()?,
        })
    }

    pub fn advance_interpolator(&mut self, context: CycleContext) {
        let last_cycle_duration = context.cycle_time.last_cycle_duration;
        let condition_input = context.condition_input;

        context.motion_safe_exits[MotionType::StandUpBack] = false;

        self.interpolator
            .advance_by(last_cycle_duration, condition_input);

        if self.interpolator.is_finished() {
            context.motion_safe_exits[MotionType::StandUpBack] = true;
        }
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let stand_up_back_estimated_remaining_duration =
            if let MotionType::StandUpBack = context.motion_selection.current_motion {
                self.advance_interpolator(context);
                Some(self.interpolator.estimated_remaining_duration())
            } else {
                self.interpolator.reset();
                None
            };
        Ok(MainOutputs {
            stand_up_back_positions: self.interpolator.value().into(),
            stand_up_back_estimated_remaining_duration: stand_up_back_estimated_remaining_duration
                .into(),
        })
    }
}
