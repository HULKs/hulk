use color_eyre::Result;
use context_attribute::context;
use framework::deserialize_not_implemented;
use framework::MainOutput;
use hardware::PathsInterface;
use motionfile::{InterpolatorState, MotionFile, MotionInterpolator};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use types::{
    condition_input::ConditionInput,
    cycle_time::CycleTime,
    joints::Joints,
    motion_selection::{MotionSafeExits, MotionSelection, MotionType},
};

#[derive(Deserialize, Serialize)]
pub struct WideStance {
    #[serde(skip, default = "deserialize_not_implemented")]
    interpolator: MotionInterpolator<Joints<f32>>,
    state: InterpolatorState<Joints<f32>>,
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
    pub wide_stance_positions: MainOutput<Joints<f32>>,
    pub wide_stance_estimated_remaining_duration: MainOutput<Option<Duration>>,
}

impl WideStance {
    pub fn new(context: CreationContext<impl PathsInterface>) -> Result<Self> {
        let paths = context.hardware_interface.get_paths();
        Ok(Self {
            interpolator: MotionFile::from_path(paths.motions.join("wide_stance.json"))?
                .try_into()?,
            state: InterpolatorState::INITIAL,
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let wide_stance_estimated_remaining_duration = if let MotionType::WideStance =
            context.motion_selection.current_motion
        {
            let last_cycle_duration = context.cycle_time.last_cycle_duration;
            let condition_input = context.condition_input;

            self.interpolator
                .advance_state(&mut self.state, last_cycle_duration, condition_input);

            Some(self.interpolator.estimated_remaining_duration(self.state))
        } else {
            self.state.reset();
            None
        };
        context.motion_safe_exits[MotionType::WideStance] = self.state.is_finished();

        Ok(MainOutputs {
            wide_stance_positions: self.interpolator.value(self.state).into(),
            wide_stance_estimated_remaining_duration: wide_stance_estimated_remaining_duration
                .into(),
        })
    }
}
