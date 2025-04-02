use color_eyre::Result;
use context_attribute::context;
use framework::deserialize_not_implemented;
use framework::MainOutput;
use hardware::PathsInterface;
use motionfile::{InterpolatorState, MotionFile, MotionInterpolator};
use serde::{Deserialize, Serialize};
use types::{
    condition_input::ConditionInput,
    cycle_time::CycleTime,
    joints::{mirror::Mirror, Joints},
    motion_selection::{MotionSafeExits, MotionSelection, MotionType},
    motor_commands::MotorCommands,
};

#[derive(Deserialize, Serialize)]
pub struct KeeperJumpLeft {
    #[serde(skip, default = "deserialize_not_implemented")]
    interpolator: MotionInterpolator<MotorCommands<Joints<f32>>>,
    state: InterpolatorState<MotorCommands<Joints<f32>>>,
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
    pub keeper_jump_left_motor_commands: MainOutput<MotorCommands<Joints<f32>>>,
}

impl KeeperJumpLeft {
    pub fn new(context: CreationContext<impl PathsInterface>) -> Result<Self> {
        let paths = context.hardware_interface.get_paths();
        Ok(Self {
            interpolator: MotionFile::from_path(paths.motions.join("keeper_jump_right.json"))?
                .try_into()?,
            state: InterpolatorState::INITIAL,
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let last_cycle_duration = context.cycle_time.last_cycle_duration;
        let condition_input = context.condition_input;

        if context.motion_selection.current_motion == MotionType::KeeperJumpLeft {
            self.interpolator
                .advance_state(&mut self.state, last_cycle_duration, condition_input);
        } else {
            self.state.reset();
        }
        context.motion_safe_exits[MotionType::KeeperJumpLeft] = self.state.is_finished();

        Ok(MainOutputs {
            keeper_jump_left_motor_commands: self.interpolator.value(self.state).mirrored().into(),
        })
    }
}
